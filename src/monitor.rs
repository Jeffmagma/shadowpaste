use arboard::Clipboard;
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use std::{thread::{self, sleep}, time};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use image::{ImageBuffer, Rgba};
use base64::{Engine as _, engine::general_purpose};
use std::io::Cursor;

#[derive(Debug, Clone, PartialEq)]
pub enum ClipboardContent {
    Text(String),
    Image(String),
    Empty
}

impl ClipboardContent {
    pub fn from_image(image: arboard::ImageData) -> Self {
        let width = image.width;
        let height = image.height;
        let bytes = image.bytes;

        if let Some(img_buffer) = ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, bytes.into_owned()) {
            let mut cursor = Cursor::new(Vec::new());
            if let Ok(_) = img_buffer.write_to(&mut cursor, image::ImageFormat::Png) {
                let base64_string = general_purpose::STANDARD.encode(cursor.into_inner());
                let final_string = format!("data:image/png;base64,{}", base64_string);
                return ClipboardContent::Image(final_string);
            }
        }

        ClipboardContent::Empty
    }
}

struct Handler {
    tx: UnboundedSender<ClipboardContent>,
    last_content: Option<ClipboardContent>,
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        sleep(time::Duration::from_millis(50)); // https://learn.microsoft.com/en-us/answers/questions/1327362/wm-clipboardupdate-issue
        let content = read_clipboard();

        // make sure new data is different from last
        if self.last_content.as_ref() == Some(&content) {
            return CallbackResult::Next;
        }

        self.last_content = Some(content.clone());
        let _ = self.tx.send(content);
        CallbackResult::Next
    }
}

fn read_clipboard() -> ClipboardContent {
    let Ok(mut clipboard) = Clipboard::new() else {
        return ClipboardContent::Empty;
    };

    if let Ok(text) = clipboard.get_text() {
        return ClipboardContent::Text(text);
    }

    if let Ok(img) = clipboard.get_image() {
        return ClipboardContent::from_image(img);
    }

    ClipboardContent::Empty
}

pub fn start_listener() -> UnboundedReceiver<ClipboardContent> {
    let (tx, rx) = mpsc::unbounded_channel();

    thread::spawn(move || {
        let handler = Handler { tx, last_content: None };
        let mut master = Master::new(handler).unwrap();
        master.run().unwrap();
    });

    rx
}