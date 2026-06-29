use libgraphics::{Canvas, Rect};
use crate::theme::Theme;
use libaipc::InputEventData;

pub trait Widget: Send {
    fn draw(&self, canvas: &mut Canvas, theme: &Theme);
    fn handle_event(&mut self, event: &InputEventData) -> bool;
    fn set_rect(&mut self, rect: Rect);
    fn get_rect(&self) -> Rect;
    fn set_focused(&mut self, focused: bool);
    fn is_focused(&self) -> bool;
}
