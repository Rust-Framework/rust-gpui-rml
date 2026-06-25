use rml_app::RmlApplication;

mod counter;

fn main() {
    RmlApplication::new()
        .title("RML Counter Demo")
        .size(px(400.), px(300.))
        .run::<counter::Counter>();
}

fn px(f: f32) -> gpui::Pixels {
    gpui::Pixels(f)
}
