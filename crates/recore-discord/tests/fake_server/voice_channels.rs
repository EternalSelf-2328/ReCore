//! Voice channels: the soundboard, moderation, when a call started, and a
//! stage's topic, speakers, audience and raised hands.

use super::*;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn sets_voice_status_as_discords_client_does() {
    let Fixture {
        _fake,
        discord,
        snapshot,
        lounge,
        ..
    } = Fixture::start("sets_voice_status_as_discords_client_does").await;
    let status = |snapshot: &Snapshot| {
        snapshot.servers[0]
            .voice_channels
            .iter()
            .find(|channel| channel.id == lounge)
            .and_then(|channel| channel.status.clone())
    };
    assert_eq!(status(&snapshot), None);
    discord
        .set_voice_status(&lounge, Some("Movie night"))
        .await
        .unwrap();
    snapshot_where(&discord, |snapshot| {
        status(snapshot).as_deref() == Some("Movie night")
    })
    .await;
    discord.set_voice_status(&lounge, None).await.unwrap();
    snapshot_where(&discord, |snapshot| status(snapshot).is_none()).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn plays_the_soundboard() {
    let Fixture {
        _fake,
        discord,
        server,
        snapshot,
        guild,
        lounge,
        bob,
        ..
    } = Fixture::start("plays_the_soundboard").await;
    let mut live = discord.live();
    let me = snapshot.user_id.clone().unwrap();
    // Discord's own sounds, asked for once, and the server's, with op 31.
    let defaults = discord.default_sounds().await.unwrap();
    assert_eq!(defaults.len(), 6);
    assert!(defaults.iter().all(|sound| sound.guild.is_none()));
    assert!(Arc::ptr_eq(
        &defaults,
        &discord.default_sounds().await.unwrap()
    ));
    discord.request_soundboard(vec![guild.clone()]);
    let sounds = live_where(&mut live, |event| match event {
        Live::SoundboardSounds { guild: of, sounds } if of == guild => Some(sounds),
        _ => None,
    })
    .await;
    let names: Vec<&str> = sounds.iter().map(|sound| sound.name.as_str()).collect();
    assert_eq!(names, ["Bob's laugh", "Applause"]);
    assert_eq!(sounds[0].emoji_id.as_deref(), Some("1400000000000000300"));

    // Playing one takes being in the call; then everyone there hears of
    // it, Alice too, with the sound's volume.
    let quack = defaults.iter().find(|sound| sound.name == "quack").unwrap();
    assert!(discord.send_sound(&lounge, quack).await.is_err());
    discord.join(&guild, &lounge).await.unwrap();
    discord.send_sound(&lounge, quack).await.unwrap();
    discord.send_sound(&lounge, &sounds[0]).await.unwrap();
    let played = |user: String| {
        move |event| match event {
            Live::SoundPlayed(played) if played.user == user => Some(played),
            _ => None,
        }
    };
    let heard = live_where(&mut live, played(me.clone())).await;
    assert_eq!(
        (heard.channel.as_str(), heard.sound.as_str()),
        (lounge.as_str(), "1")
    );
    let heard = live_where(&mut live, played(me.clone())).await;
    assert_eq!(
        (heard.sound.as_str(), heard.volume),
        (sounds[0].id.as_str(), 0.8)
    );
    // As Discord's client sends them: the server only for a server's sound.
    let sent: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/soundboard-sends", "")).unwrap();
    assert_eq!(sent[1]["sound_id"], "1");
    assert_eq!(sent[1]["emoji_name"], "🦆");
    assert!(sent[1].get("source_guild_id").is_none(), "{sent}");
    assert_eq!(sent[2]["source_guild_id"], guild.as_str());
    assert_eq!(sent[2]["emoji_id"], "1400000000000000300");

    // Bob plays one; its file comes from the CDN, Ogg for the server's.
    post_control(server, "/_control/soundboard-play?user=bob", "");
    let heard = live_where(&mut live, played(bob.clone())).await;
    let file = discord.sound_file(&heard.sound).await.unwrap();
    assert!(file.starts_with(b"OggS"));
    assert!(discord.sound_file(&quack.id).await.unwrap().len() > 1000);

    // A sound added to the server arrives on its own.
    let added = control(server, "POST", "/_control/soundboard-add?name=Bell", "");
    let sound = live_where(&mut live, |event| match event {
        Live::SoundboardSound(sound) => Some(sound),
        _ => None,
    })
    .await;
    assert_eq!(
        (sound.id.as_str(), sound.name.as_str()),
        (added.trim(), "Bell")
    );
    assert_eq!(sound.guild.as_deref(), Some(guild.as_str()));
    discord.leave(&guild);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn keeps_the_soundboards_settings() {
    let Fixture {
        _fake,
        discord,
        server,
        snapshot,
        guild,
        lounge,
        ..
    } = Fixture::start("keeps_the_soundboards_settings").await;
    let mut live = discord.live();
    let me = snapshot.user_id.clone().unwrap();
    assert_eq!(snapshot.soundboard_volume, None);
    assert!(snapshot.join_sounds.is_empty());

    // The favourites, in their order, and changed, which every session
    // hears of.
    assert_eq!(
        discord.favorite_sounds().await.unwrap(),
        ["1400000000000000321", "2"]
    );
    discord
        .set_favorite_sounds(&["2".to_owned(), "1".to_owned()])
        .await
        .unwrap();
    let favorites = live_where(&mut live, |event| match event {
        Live::FavoriteSounds(favorites) => Some(favorites),
        _ => None,
    })
    .await;
    assert_eq!(favorites, ["2", "1"]);
    assert_eq!(discord.favorite_sounds().await.unwrap(), ["2", "1"]);

    // The volume, and a join sound for the server and one for every
    // server, keeping the custom status beside them.
    discord.set_soundboard_volume(42.5).await.unwrap();
    discord
        .set_join_sound(Some(&guild), Some(("1400000000000000320", Some(&guild))))
        .await
        .unwrap();
    discord
        .set_join_sound(None, Some(("2", None)))
        .await
        .unwrap();
    let snapshot = snapshot_where(&discord, |snapshot| snapshot.join_sounds.len() == 2).await;
    assert_eq!(snapshot.soundboard_volume, Some(42.5));
    assert_eq!(
        snapshot
            .custom_status
            .as_ref()
            .map(|custom| custom.text.as_str()),
        Some("Testing ReCore")
    );

    // Joining with Nitro, the server's own plays for the others.
    post_control(server, "/_control/nitro", "");
    discord.join(&guild, &lounge).await.unwrap();
    discord.play_join_sound(&lounge).await.unwrap();
    let played = live_where(&mut live, |event| match event {
        Live::SoundPlayed(played) if played.user == me => Some(played),
        _ => None,
    })
    .await;
    assert_eq!(played.sound, "1400000000000000320");
    let asked: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/custom-call-sounds", "")).unwrap();
    let animation = asked[0]["animation_id"].as_u64().unwrap();
    assert!((10..=15).contains(&animation), "{asked}");

    // Taken away, the one for every server is left.
    discord.set_join_sound(Some(&guild), None).await.unwrap();
    let snapshot = snapshot_where(&discord, |snapshot| snapshot.join_sounds.len() == 1).await;
    assert_eq!(snapshot.join_sounds[0].server, None);
    assert_eq!(snapshot.join_sounds[0].sound, "2");
    discord.leave(&guild);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn moderates_voice() {
    let Fixture {
        _fake,
        discord,
        server,
        snapshot,
        guild,
        lounge,
        ..
    } = Fixture::start("moderates_voice").await;
    let create_room = snapshot
        .servers
        .iter()
        .flat_map(|server| &server.voice_channels)
        .find(|found| found.name == "Create Room")
        .unwrap()
        .id
        .clone();
    // Alice's Staff role may mute, deafen and move members in voice, as
    // the voice menu does: Dave, muted and deafened by himself, is server
    // muted and deafened, then not, moved to Create Room and disconnected.
    let dave_in = |channel: &str| {
        let channel = channel.to_owned();
        move |snapshot: &Snapshot| {
            snapshot.voice.get(&channel).and_then(|members| {
                members
                    .iter()
                    .find(|member| member.name == "Dave")
                    .map(|member| (member.server_muted, member.server_deafened, member.muted))
            })
        }
    };
    let dave = snapshot_where(&discord, |snapshot| dave_in(&lounge)(snapshot).is_some())
        .await
        .voice[&lounge]
        .iter()
        .find(|member| member.name == "Dave")
        .unwrap()
        .user_id
        .clone();
    discord.server_mute(&guild, &dave, true).await.unwrap();
    snapshot_where(&discord, |snapshot| {
        dave_in(&lounge)(snapshot) == Some((true, false, true))
    })
    .await;
    discord.server_deafen(&guild, &dave, true).await.unwrap();
    snapshot_where(&discord, |snapshot| {
        dave_in(&lounge)(snapshot) == Some((true, true, true))
    })
    .await;
    discord.server_mute(&guild, &dave, false).await.unwrap();
    discord.server_deafen(&guild, &dave, false).await.unwrap();
    // Still muted by himself.
    snapshot_where(&discord, |snapshot| {
        dave_in(&lounge)(snapshot) == Some((false, false, true))
    })
    .await;
    discord
        .move_member(&guild, &dave, Some(&create_room))
        .await
        .unwrap();
    snapshot_where(&discord, |snapshot| {
        dave_in(&create_room)(snapshot).is_some() && dave_in(&lounge)(snapshot).is_none()
    })
    .await;
    discord.move_member(&guild, &dave, None).await.unwrap();
    snapshot_where(&discord, |snapshot| {
        dave_in(&create_room)(snapshot).is_none()
    })
    .await;
    discord
        .server_mute(&guild, &dave, true)
        .await
        .expect_err("he isn't in voice");
    let changes: serde_json::Value = serde_json::from_str(&crate::control(
        server,
        "GET",
        "/_control/voice-changes",
        "",
    ))
    .unwrap();
    assert_eq!(
        changes,
        serde_json::json!([
            {"user": dave, "mute": true},
            {"user": dave, "deaf": true},
            {"user": dave, "mute": false},
            {"user": dave, "deaf": false},
            {"user": dave, "channel_id": create_room},
            {"user": dave, "channel_id": null},
        ])
    );
    post_control(server, "/_control/voice-reset", "");
    snapshot_where(&discord, |snapshot| dave_in(&lounge)(snapshot).is_some()).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn speaks_in_a_stage() {
    let Fixture {
        _fake,
        discord,
        server,
        snapshot,
        guild,
        ..
    } = Fixture::start("speaks_in_a_stage").await;
    let stage = snapshot.servers[0]
        .voice_channels
        .iter()
        .find(|channel| channel.name == "Stage")
        .unwrap()
        .id
        .clone();
    let snapshot = snapshot_where(&discord, |snapshot| {
        snapshot.stage_instances.contains_key(&stage) && snapshot.voice.contains_key(&stage)
    })
    .await;
    // Its stage instance, and who speaks and listens there: Erin speaks,
    // and Frank listens with his hand raised.
    let found = &snapshot.stage_instances[&stage];
    assert_eq!(found.topic, "Community AMA");
    assert_eq!(found.privacy_level, 2);
    assert_eq!(discord.stage_instance(&stage).await.unwrap(), *found);
    let people = &snapshot.voice[&stage];
    let person = |name: &str| people.iter().find(|person| person.name == name).unwrap();
    assert!(!person("Erin").suppressed && !person("Erin").requested);
    assert!(person("Frank").suppressed && person("Frank").requested);
    let me = snapshot.user_id.clone().unwrap();

    // Its topic changes, as Stage Topic does, and STAGE_INSTANCE_UPDATE
    // shows it.
    discord
        .set_stage_topic(&stage, "Q&A with the team")
        .await
        .unwrap();
    let snapshot = snapshot_where(&discord, |snapshot| {
        snapshot.stage_instances[&stage].topic == "Q&A with the team"
    })
    .await;
    assert_eq!(snapshot.stage_instances[&stage].privacy_level, 2);
    let changes: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/stage-changes", "")).unwrap();
    assert_eq!(changes, serde_json::json!([{"topic": "Q&A with the team"}]));

    // Joining, this account listens, raises its hand and takes it back.
    discord.join(&guild, &stage).await.unwrap();
    let in_stage = |snapshot: &Snapshot| {
        snapshot.voice[&stage]
            .iter()
            .find(|person| person.user_id == me)
            .map(|person| (person.suppressed, person.requested))
    };
    snapshot_where(&discord, |snapshot| in_stage(snapshot).is_some()).await;
    discord.set_suppressed(&guild, &me, true).await.unwrap();
    snapshot_where(&discord, |snapshot| {
        in_stage(snapshot) == Some((true, false))
    })
    .await;
    discord.request_to_speak(&guild, true).await.unwrap();
    snapshot_where(&discord, |snapshot| {
        in_stage(snapshot) == Some((true, true))
    })
    .await;
    discord.request_to_speak(&guild, false).await.unwrap();
    snapshot_where(&discord, |snapshot| {
        in_stage(snapshot) == Some((true, false))
    })
    .await;

    // A moderator moves it between the audience and the speakers.
    discord.set_suppressed(&guild, &me, false).await.unwrap();
    snapshot_where(&discord, |snapshot| {
        in_stage(snapshot) == Some((false, false))
    })
    .await;
    discord.set_suppressed(&guild, &me, true).await.unwrap();
    snapshot_where(&discord, |snapshot| {
        in_stage(snapshot) == Some((true, false))
    })
    .await;
    let changes: serde_json::Value =
        serde_json::from_str(&control(server, "GET", "/_control/voice-state-changes", "")).unwrap();
    let changes = changes.as_array().unwrap();
    assert_eq!(changes[0]["suppress"], true);
    assert!(changes[1]["request_to_speak_timestamp"].is_string());
    assert_eq!(
        changes[2]["request_to_speak_timestamp"],
        serde_json::Value::Null
    );
    assert_eq!(changes[3]["suppress"], false);
    assert_eq!(changes[4]["suppress"], true);
    discord.leave(&guild);

    // The stage ends and starts again, as its menu does.
    post_control(server, "/_control/stage-instance?off", "");
    snapshot_where(&discord, |snapshot| {
        !snapshot.stage_instances.contains_key(&stage)
    })
    .await;
    assert!(discord.stage_instance(&stage).await.is_err());
    post_control(server, "/_control/stage-instance", "");
    let snapshot = snapshot_where(&discord, |snapshot| {
        snapshot.stage_instances.contains_key(&stage)
    })
    .await;
    assert_eq!(snapshot.stage_instances[&stage].topic, "Community AMA");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts testing/fake_discord.ts with bun; run scripts/test-fake-discord"]
async fn knows_when_voice_channels_calls_started() {
    let Fixture {
        _fake,
        discord,
        server,
        snapshot,
        guild,
        lounge,
        ..
    } = Fixture::start("knows_when_voice_channels_calls_started").await;
    let create_room = snapshot
        .servers
        .iter()
        .flat_map(|server| &server.voice_channels)
        .find(|found| found.name == "Create Room")
        .unwrap()
        .id
        .clone();
    let now = || {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    };
    // Opening the server asks once when its voice channels' calls started:
    // the Lounge's, 47 minutes before the fake started.
    discord.view(&guild, None);
    let snapshot = snapshot_where(&discord, |snapshot| {
        snapshot.voice_started.contains_key(&lounge)
    })
    .await;
    let started = snapshot.voice_started[&lounge];
    assert!(
        (46 * 60..=60 * 60).contains(&(now() - started)),
        "{started}"
    );
    assert!(!snapshot.voice_started.contains_key(&create_room));
    discord.view(&guild, Some(&lounge));

    // Someone joining an empty channel starts its call, and the last to
    // leave ends it, as VOICE_CHANNEL_START_TIME_UPDATE says.
    let dave = snapshot.voice[&lounge]
        .iter()
        .find(|member| member.name == "Dave")
        .unwrap()
        .user_id
        .clone();
    discord
        .move_member(&guild, &dave, Some(&create_room))
        .await
        .unwrap();
    let snapshot = snapshot_where(&discord, |snapshot| {
        snapshot.voice_started.contains_key(&create_room)
    })
    .await;
    assert!(now() - snapshot.voice_started[&create_room] < 60);
    assert_eq!(
        snapshot.voice_started[&lounge], started,
        "the Lounge's goes on"
    );
    discord.move_member(&guild, &dave, None).await.unwrap();
    snapshot_where(&discord, |snapshot| {
        !snapshot.voice_started.contains_key(&create_room)
    })
    .await;
    // Viewing another of its channels, after all that, asked nothing more.
    let asked = || -> serde_json::Value {
        serde_json::from_str(&control(server, "GET", "/_control/channel-info", "")).unwrap()
    };
    assert_eq!(asked(), serde_json::json!([guild]));
    post_control(server, "/_control/voice-reset", "");

    // A resumed session missed nothing, and asks nothing again.
    post_control(server, "/_control/gateway-reconnect", "");
    timeout(WAIT, async {
        while control(server, "GET", "/_control/sessions", "") != r#"{"identifies":1,"resumes":1}"#
        {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("the session didn't resume");
    discord.set_status("idle").await.unwrap();
    presence_where(server, |presence| presence["status"] == "idle").await;
    discord.set_status("online").await.unwrap();
    assert_eq!(asked(), serde_json::json!([guild]));
    assert!(
        discord
            .snapshots()
            .borrow()
            .voice_started
            .contains_key(&lounge)
    );

    // Leaving takes its times; joined again, the server on screen is asked
    // again, once.
    discord.leave_server(&guild).await.unwrap();
    snapshot_where(&discord, |snapshot| snapshot.voice_started.is_empty()).await;
    post_control(server, "/_control/rejoin", "");
    let snapshot = snapshot_where(&discord, |snapshot| {
        snapshot.voice_started.get(&lounge) == Some(&started)
    })
    .await;
    assert!(snapshot.servers.iter().any(|known| known.id == guild));
    assert_eq!(asked(), serde_json::json!([guild, guild]));
}
