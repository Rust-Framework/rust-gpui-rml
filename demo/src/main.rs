use gpui::px;
use rml_app::RmlApplication;

#[path = "counter.rml.rs"]
mod counter;

fn main() {
    RmlApplication::new()
        .title("RML Counter Demo")
        .size(px(400.), px(300.))
        .run::<counter::Counter>();
}
