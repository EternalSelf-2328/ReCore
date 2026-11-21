//! The way a popup opens and closes: over [`DURATION`], eased out, its
//! card grows from [`FROM`] of its size to all of it about its centre
//! ([`Grow`]) and the window's dim under it fades in ([`dim`]), and closing
//! does the same the other way. A layer that opens is wrapped in
//! [`Motion`], which says how far open it is to what's drawn inside it
//! while it draws, and asks for the window's redraws until it's open;
//! nothing sends a message or builds the view anew for it. One closing
//! ([`leave`]) is drawn and takes nothing, neither the pointer nor the
//! keys, and says once it's gone. Outside a `Motion` a card is drawn as it
//! is.

use std::{
    cell::Cell,
    time::{Duration, Instant},
};

use iced::{
    Color, Element, Transformation,
    advanced::{Renderer as _, Shell, renderer, widget::Operation},
    mouse, window,
};

use super::{At, Hooks, Inner, Renderer, Stateless, Wrap};

/// How long a popup takes to open.
pub const DURATION: Duration = Duration::from_millis(190);
/// How large a card opens from.
pub const FROM: f32 = 0.8;

thread_local! {
    /// How far open the layer being drawn is, eased, from 0 to 1: set by
    /// the [`Motion`] drawing it, 1 outside one.
    static SHOWN: Cell<f32> = const { Cell::new(1.0) };
    /// Whether popups move at all, as ReCord's motion setting says.
    static MOVING: Cell<bool> = const { Cell::new(true) };
    /// The duration and step count a CSS theme's transition asks for;
    /// ReCore's own are 190 ms in four steps.
    static PACE: Cell<(Duration, u32)> = const { Cell::new((DURATION, 4)) };
}

/// Turns popup motion on or off, as ReCord's `--rl-motion` does.
pub fn set_moving(moving: bool) {
    MOVING.set(moving);
}

/// Sets how long a popup opens for and how many steps it moves in, as a CSS
/// theme's transition on a menu, popout or dialog does. Zero steps follows
/// the curve continuously, as CSS's `ease` and `linear` do; ReCore's own
/// motion is four steps.
pub fn set_pace(duration: Duration, steps: u32) {
    PACE.set((duration, steps.min(16)));
}

fn moving() -> bool {
    MOVING.get()
}

fn pace() -> (Duration, u32) {
    PACE.get()
}

/// How far open a layer is that opened at `start`, at `now`, eased out
/// on a quadratic curve.
pub fn shown(start: Instant, now: Instant) -> f32 {
    let (duration, steps) = pace();
    let t = (now.saturating_duration_since(start).as_secs_f32() / duration.as_secs_f32()).min(1.0);
    // ReCord moves menus in four discrete steps, not continuously; a theme
    // that names `ease` or `linear` gets the continuous curve.
    let t = if steps > 0 {
        (t * steps as f32).floor() / steps as f32
    } else {
        t
    };
    1.0 - (1.0 - t) * (1.0 - t)
}

/// For a layer closing since `start`, when it would have started opening to
/// be as far open at `now` as it is: a popup opened again as it closes
/// grows on from there.
pub fn reopened(start: Instant, now: Instant) -> Instant {
    let (duration, steps) = pace();
    let open = 1.0 - shown(start, now);
    // `shown` backwards, on its stepped grid: the time at which it's
    // nearest to `open`.
    let t = 1.0 - (1.0 - open).sqrt();
    let t = if steps > 0 {
        (t * steps as f32).floor() / steps as f32
    } else {
        t
    };
    now - duration.mul_f32(t)
}

/// A layer opening or closing: `start` is when it was first drawn so,
/// kept by the one who keeps what's open, as the widget tree's state
/// would go with a layer opened or closed under it.
pub struct Motion<'a, M> {
    start: &'a Cell<Option<Instant>>,
    /// While it closes, what it says once it's closed.
    closed: Option<M>,
}

impl<M> Motion<'_, M> {
    /// How far open it is at `now`, its start set if it had none.
    fn shown(&self, now: Instant) -> f32 {
        if !moving() {
            return if self.closed.is_some() { 0.0 } else { 1.0 };
        }
        let start = *self.start.get().get_or_insert(now);
        self.start.set(Some(start));
        let opened = shown(start, now);
        if self.closed.is_some() {
            1.0 - opened
        } else {
            opened
        }
    }
}

/// `layer`, opening since it was first drawn (`start`, empty until then).
pub fn motion<'a, M: Clone + 'a>(
    layer: impl Into<Element<'a, M>>,
    start: &'a Cell<Option<Instant>>,
) -> Element<'a, M> {
    Wrap::new(
        layer,
        Motion {
            start,
            closed: None,
        },
    )
    .into()
}

/// `layer`, closed and closing since it was first drawn so (`start`):
/// drawn, it takes nothing, and says `closed` as it's gone.
pub fn leave<'a, M: Clone + 'a>(
    layer: impl Into<Element<'a, M>>,
    start: &'a Cell<Option<Instant>>,
    closed: M,
) -> Element<'a, M> {
    Wrap::new(
        layer,
        Motion {
            start,
            closed: Some(closed),
        },
    )
    .into()
}

impl<M: Clone> Hooks<M> for Motion<'_, M> {
    type State = Stateless;

    fn update(
        &mut self,
        _state: &mut Stateless,
        event: &iced::Event,
        _at: At,
        shell: &mut Shell<'_, M>,
        child: &mut Inner<'_, '_, M>,
    ) {
        if !moving() {
            if let Some(closed) = &self.closed {
                shell.publish(closed.clone());
            }
            return;
        }
        if let iced::Event::Window(window::Event::RedrawRequested(now)) = event {
            let start = *self.start.get().get_or_insert(*now);
            self.start.set(Some(start));
            if now.saturating_duration_since(start) < DURATION {
                shell.request_redraw();
            } else if let Some(closed) = &self.closed {
                shell.publish(closed.clone());
            }
        }
        if self.closed.is_none() {
            child.update(shell);
        }
    }

    fn interaction(
        &self,
        _state: &Stateless,
        _at: At,
        child: mouse::Interaction,
    ) -> mouse::Interaction {
        // Closing, the pointer goes to what's under it.
        if self.closed.is_some() {
            mouse::Interaction::None
        } else {
            child
        }
    }

    fn draw(
        &self,
        _state: &Stateless,
        renderer: &mut Renderer,
        _at: At,
        child: &mut dyn FnMut(&mut Renderer),
    ) {
        let shown = self.shown(Instant::now());
        if shown <= 0.0 {
            return;
        }
        let outer = SHOWN.replace(shown);
        child(renderer);
        SHOWN.set(outer);
    }

    fn overlays(&self) -> bool {
        self.closed.is_none()
    }

    fn operate(
        &mut self,
        _state: &mut Stateless,
        operation: &mut dyn Operation,
        child: &mut dyn FnMut(&mut dyn Operation),
    ) {
        // Closing, no focus or scroll finds what's in it.
        if self.closed.is_none() {
            child(operation);
        }
    }
}

/// A popup's card, grown from [`FROM`] of its size about its centre as
/// the [`Motion`] it's drawn in opens.
pub struct Grow;

/// `card`, grown as its layer opens.
pub fn grow<'a, M: 'a>(card: impl Into<Element<'a, M>>) -> Element<'a, M> {
    Wrap::new(card, Grow).into()
}

impl<M> Hooks<M> for Grow {
    type State = Stateless;

    fn draw(
        &self,
        _state: &Stateless,
        renderer: &mut Renderer,
        at: At,
        child: &mut dyn FnMut(&mut Renderer),
    ) {
        let shown = SHOWN.get();
        if shown >= 1.0 {
            child(renderer);
            return;
        }
        let scale = FROM + (1.0 - FROM) * shown;
        let centre = at.child.center();
        let transformation = Transformation::translate(centre.x, centre.y)
            * Transformation::scale(scale)
            * Transformation::translate(-centre.x, -centre.y);
        renderer.with_transformation(transformation, |renderer| child(renderer));
    }
}

/// The window dimmed by `color` under a popup, fading in as its layer
/// opens: `layer` (taking the clicks, holding the id) is drawn over it.
pub fn dim<'a, M: 'a>(layer: impl Into<Element<'a, M>>, color: Color) -> Element<'a, M> {
    Wrap::new(layer, Dim { color }).into()
}

/// What [`dim`] paints.
pub struct Dim {
    color: Color,
}

impl<M> Hooks<M> for Dim {
    type State = Stateless;

    fn draw(
        &self,
        _state: &Stateless,
        renderer: &mut Renderer,
        at: At,
        child: &mut dyn FnMut(&mut Renderer),
    ) {
        let color = Color {
            a: self.color.a * SHOWN.get(),
            ..self.color
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds: at.bounds,
                ..Default::default()
            },
            color,
        );
        child(renderer);
    }
}

/// ReCord's `rl-power-on`: the whole screen unfolds from a bright
/// horizontal line into its full height over 620 ms, in nine steps.
pub const POWER: Duration = Duration::from_millis(620);

/// `layer`, unfolding as it opens after the boot signal.
pub fn power_on<'a, M: Clone + 'a>(
    layer: impl Into<Element<'a, M>>,
    start: Instant,
) -> Element<'a, M> {
    Wrap::new(layer, PowerOn { start }).into()
}

/// What [`power_on`] paints.
struct PowerOn {
    start: Instant,
}

impl<M: Clone> Hooks<M> for PowerOn {
    type State = Stateless;

    fn update(
        &mut self,
        _state: &mut Stateless,
        event: &iced::Event,
        _at: At,
        shell: &mut Shell<'_, M>,
        _child: &mut Inner<'_, '_, M>,
    ) {
        if let iced::Event::Window(window::Event::RedrawRequested(_)) = event
            && self.start.elapsed() < POWER
        {
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        _state: &Stateless,
        renderer: &mut Renderer,
        at: At,
        child: &mut dyn FnMut(&mut Renderer),
    ) {
        let t = (self.start.elapsed().as_secs_f32() / POWER.as_secs_f32()).min(1.0);
        let step = (t * 9.0).floor() / 9.0;
        if step >= 1.0 {
            child(renderer);
            return;
        }
        let scale_x = if step < 0.35 {
            0.35 + 0.65 * (step / 0.35)
        } else {
            1.0
        };
        let scale_y = if step < 0.35 {
            0.004
        } else if step < 0.65 {
            0.004 + 0.996 * ((step - 0.35) / 0.30)
        } else {
            1.0
        };
        // Clip, rather than squash, the screen: the power-on line widens
        // to the full window first, then opens its height.
        let width = at.bounds.width * scale_x;
        let height = at.bounds.height * scale_y;
        let bounds = iced::Rectangle {
            x: at.bounds.x + (at.bounds.width - width) / 2.0,
            y: at.bounds.y + (at.bounds.height - height) / 2.0,
            width,
            height,
        };
        renderer.with_layer(bounds, |renderer| child(renderer));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_popup_opens_eased_out_over_its_duration() {
        let start = Instant::now();
        assert_eq!(shown(start, start), 0.0);
        let half = shown(start, start + DURATION / 2);
        // Eased out: past half way at half the time.
        assert!((half - 0.75).abs() < 1e-4, "{half}");
        assert_eq!(shown(start, start + DURATION), 1.0);
        assert_eq!(shown(start, start + DURATION * 3), 1.0);
    }

    #[test]
    fn a_popup_opened_again_as_it_closes_grows_on_from_where_it_was() {
        let closing = Instant::now();
        let now = closing + DURATION / 4;
        let open = 1.0 - shown(closing, now);
        let again = reopened(closing, now);
        assert!((shown(again, now) - open).abs() < 0.2);
        // Closed all the way, it opens from the start.
        let gone = closing + DURATION;
        assert!(shown(reopened(closing, gone), gone) < 1e-3);
    }

    #[test]
    fn a_css_themes_transition_sets_the_pace() {
        let start = Instant::now();
        set_pace(Duration::from_millis(240), 0);
        assert_eq!(shown(start, start + Duration::from_millis(120)), 0.75);
        assert_eq!(shown(start, start + Duration::from_millis(240)), 1.0);
        assert_eq!(shown(start, start + Duration::from_millis(60)), 28.0 / 64.0);
        set_pace(DURATION, 4);
    }
}
