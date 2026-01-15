use arboard::Clipboard;
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use crossbeam_channel::{unbounded, Receiver};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use iced::{
    keyboard::{Event as KeyEvent, KeyCode},
    subscription,
    window::{Level, PlatformSpecific},
    Application, Command, Element, Font, Length, Settings, Subscription, Theme,
};
use std::{collections::VecDeque, sync::Arc, thread};

const MAX_HISTORY: usize = 10;

#[derive(Debug, Clone)]
enum Message {
    ShowWindow,
    NumberPressed(usize),
    ClipboardUpdated(String),
    CheckClipboard,
    ConfirmSelection,
    Hide,
    EventOccurred(iced::Event),
    ClipboardError(String),
}

struct Handler {
    tx: Arc<crossbeam_channel::Sender<u8>>,
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        self.tx.send(1).unwrap();
        CallbackResult::Next
    }
}

struct ClipboardManager {
    history: VecDeque<String>,
    current_selection: Option<usize>,
    visible: bool,
    hotkey_receiver: Receiver<u8>,
    clipboard: Option<Clipboard>,
}

impl ClipboardManager {
    fn new(hotkey_receiver: Receiver<u8>) -> Self {
        Self {
            history: VecDeque::with_capacity(MAX_HISTORY),
            current_selection: None,
            visible: false,
            hotkey_receiver,
            clipboard: Clipboard::new().ok(),
        }
    }

    fn add_to_history(&mut self, content: String) {
        for (i, entry) in self.history.clone().iter().enumerate() {
            if content == *entry {
                let e = self.history.remove(i);
                self.history.push_front(e.unwrap());
                return;
            }
        }
        if self.history.len() >= MAX_HISTORY {
            self.history.pop_back();
        }
        self.history.push_front(content);
    }

    fn check_clipboard(&mut self) -> Command<Message> {
        if let Some(clipboard) = &mut self.clipboard {
            match clipboard.get_text() {
                Ok(text) => {
                    Command::perform(async move { Message::ClipboardUpdated(text) }, |msg| msg)
                }
                Err(e) => Command::perform(
                    async move { Message::ClipboardError(e.to_string()) },
                    |msg| msg,
                ),
            }
        } else {
            Command::none()
        }
    }

    fn set_clipboard_content(&mut self, content: String) -> Command<Message> {
        if let Some(clipboard) = &mut self.clipboard {
            match clipboard.set_text(content) {
                Ok(_) => Command::none(),
                Err(e) => Command::perform(
                    async move { Message::ClipboardError(e.to_string()) },
                    |msg| msg,
                ),
            }
        } else {
            Command::none()
        }
    }
}

impl Application for ClipboardManager {
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = Receiver<u8>;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let mut manager = Self::new(flags);
        let command = manager.check_clipboard();
        (manager, command)
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }

    fn title(&self) -> String {
        String::from("clipzero")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ShowWindow => {
                self.visible = true;
                self.current_selection = Some(0);
                Command::batch(vec![
                    self.check_clipboard(),
                    iced::window::gain_focus(),
                    iced::window::change_mode(iced::window::Mode::Windowed),
                ])
            }
            Message::CheckClipboard => {
                self.current_selection = Some(0);
                self.check_clipboard()
            }
            Message::Hide => {
                self.visible = false;
                self.current_selection = None;
                iced::window::change_mode(iced::window::Mode::Hidden)
            }
            Message::NumberPressed(num) => {
                if self.visible {
                    self.current_selection = Some(num);
                }
                Command::none()
            }
            Message::ClipboardUpdated(content) => {
                self.add_to_history(content);
                Command::none()
            }
            Message::ConfirmSelection => {
                if let Some(index) = self.current_selection {
                    if let Some(content) = self.history.get(index) {
                        self.visible = false;
                        return Command::batch(vec![
                            self.set_clipboard_content(content.clone()),
                            iced::window::change_mode(iced::window::Mode::Hidden),
                        ]);
                    }
                }
                Command::none()
            }
            Message::EventOccurred(event) => {
                if let iced::Event::Keyboard(key_event) = event {
                    match key_event {
                        KeyEvent::KeyPressed { key_code, .. } => {
                            if self.visible {
                                return match key_code {
                                    KeyCode::Escape => self.update(Message::Hide),
                                    KeyCode::Enter => self.update(Message::ConfirmSelection),
                                    _ => {
                                        if let Some(num) = key_code_to_number(key_code) {
                                            return self.update(Message::NumberPressed(num));
                                        }
                                        return Command::none();
                                    }
                                };
                            };
                        }
                        _ => return Command::none(),
                    }
                }
                Command::none()
            }
            Message::ClipboardError(error) => {
                eprintln!("Clipboard error occured {}", error);
                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            subscription::events().map(Message::EventOccurred),
            subscription::unfold(
                "hotkey-subscription",
                self.hotkey_receiver.clone(),
                |receiver| async move {
                    if let Ok(value) = receiver.recv() {
                        if value == 1 {
                            // sentinel for check clipboard
                            return (Some(Message::CheckClipboard), receiver);
                        }
                        (Some(Message::ShowWindow), receiver)
                    } else {
                        (None, receiver)
                    }
                },
            )
            .map(|opt| opt.unwrap()),
        ])
    }

    fn view(&self) -> Element<Message> {
        use iced::widget::{column, container, text};

        let content = if self.visible {
            let mut items = column![].spacing(10).padding(20);

            if self.history.is_empty() {
                items = items.push(text("No clipboard history yet"));
            }

            let curr = self.current_selection.unwrap_or(0);
            if curr > self.history.len()
                || self
                    .history
                    .get(curr)
                    .unwrap_or(&String::from(""))
                    .is_empty()
            {
                items = items.push(text("Out of range of stored history."))
            } else {
                let mut content = self.history.get(curr).expect("help").clone();
                if content.chars().count() > 100 {
                    content = content.chars().take(100).collect();
                    content.push_str("...");
                }
                items = items.push(text(content));
            }

            items
        } else {
            column![text("")]
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

fn key_code_to_number(key_code: KeyCode) -> Option<usize> {
    match key_code {
        KeyCode::Key1 => Some(0),
        KeyCode::Key2 => Some(1),
        KeyCode::Key3 => Some(2),
        KeyCode::Key4 => Some(3),
        KeyCode::Key5 => Some(4),
        KeyCode::Key6 => Some(5),
        KeyCode::Key7 => Some(6),
        KeyCode::Key8 => Some(7),
        KeyCode::Key9 => Some(8),
        KeyCode::Key0 => Some(9),
        _ => None,
    }
}

fn main() -> iced::Result {
    let manager = GlobalHotKeyManager::new().unwrap();
    let (tx, rx) = unbounded();

    let arctx = Arc::new(tx);

    let arctx_event = Arc::clone(&arctx);
    let arctx_monitor = Arc::clone(&arctx);

    let hotkey_open = HotKey::new(Some(Modifiers::SUPER), Code::Digit0);
    manager.register(hotkey_open).unwrap();

    thread::spawn(move || {
        for _event in GlobalHotKeyEvent::receiver() {
            arctx_event.clone().send(0).unwrap()
        }
    });

    let handler = Handler { tx: arctx_monitor };
    thread::spawn(move || {
        let _ = Master::new(handler).run();
    });

    let settings = Settings {
        id: Some(String::from("clipzero")),
        window: iced::window::Settings {
            size: (400, 200),
            position: iced::window::Position::Specific(0, 0),
            min_size: None,
            max_size: None,
            visible: false,
            resizable: false,
            decorations: false,
            transparent: false,
            level: Level::AlwaysOnTop,
            icon: None,
            platform_specific: PlatformSpecific::default(),
        },
        flags: rx,
        default_font: Font::MONOSPACE,
        default_text_size: 20.0,
        antialiasing: false,
        exit_on_close_request: false,
    };

    ClipboardManager::run(settings)
}
