pub mod options;

use self::options::{MenuItemOption, NativeLabel};
use tao::menu::{MenuId, MenuItem, MenuItemAttributes};

pub fn build_native_item(label: &NativeLabel) -> MenuItem {
    match label {
        NativeLabel::Hide => MenuItem::Hide,
        NativeLabel::Services => MenuItem::Services,
        NativeLabel::HideOthers => MenuItem::HideOthers,
        NativeLabel::ShowAll => MenuItem::ShowAll,
        NativeLabel::CloseWindow => MenuItem::CloseWindow,
        NativeLabel::Quit => MenuItem::Quit,
        NativeLabel::Copy => MenuItem::Copy,
        NativeLabel::Cut => MenuItem::Cut,
        NativeLabel::Undo => MenuItem::Undo,
        NativeLabel::Redo => MenuItem::Redo,
        NativeLabel::SelectAll => MenuItem::SelectAll,
        NativeLabel::Paste => MenuItem::Paste,
        NativeLabel::EnterFullScreen => MenuItem::EnterFullScreen,
        NativeLabel::Minimize => MenuItem::Minimize,
        NativeLabel::Zoom => MenuItem::Zoom,
        NativeLabel::Separator => MenuItem::Separator,
    }
}
