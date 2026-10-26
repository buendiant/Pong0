//! Configuración del cliente y formulario de conexión; no realiza operaciones de red.
use macroquad::prelude::*;

use crate::client::network::NetworkConfig;

pub const CLIENT_USAGE: &str = "Uso:\n  client                         Abrir formulario de conexión\n  client <SERVIDOR:PUERTO> <NICKNAME> <EMAIL>\n  client --demo                  Jugar la demo local\n  client --help                  Mostrar esta ayuda";

pub enum ClientMode {
    Form,
    Online(NetworkConfig),
    Demo,
    Help,
}

pub fn parse_client_args(args: &[String]) -> Result<ClientMode, String> {
    match args {
        [] => Ok(ClientMode::Form),
        [flag] if flag == "--demo" => Ok(ClientMode::Demo),
        [flag] if flag == "--help" || flag == "-h" => Ok(ClientMode::Help),
        [server, nickname, email] => {
            NetworkConfig::new(server.clone(), nickname.clone(), email.clone())
                .map(ClientMode::Online)
        }
        _ => Err("Indica servidor, nickname y correo, o inicia sin argumentos.".into()),
    }
}

/// El formulario devuelve datos validados; el socket lo crea el trabajador de red.
pub async fn connection_form(initial_error: Option<String>) -> Option<NetworkConfig> {
    let mut fields = [
        TextField::new("127.0.0.1:8080"),
        TextField::new("Jugador"),
        TextField::new(""),
    ];
    let mut focused = 0;
    fields[0].selected = true;
    let mut error = initial_error;
    prevent_quit();

    loop {
        if is_quit_requested() || is_key_pressed(KeyCode::Escape) {
            return None;
        }
        let scale = (screen_width() / 800.0)
            .min(screen_height() / 600.0)
            .max(0.1);
        let origin = vec2(
            (screen_width() - 800.0 * scale) * 0.5,
            (screen_height() - 600.0 * scale) * 0.5,
        );
        let point = |x, y| origin + vec2(x, y) * scale;
        let mouse = vec2(mouse_position().0, mouse_position().1);
        let mut connect = is_key_pressed(KeyCode::Enter);

        if is_key_pressed(KeyCode::Tab) {
            let backwards = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            fields[focused].selected = false;
            focused = (focused + if backwards { 2 } else { 1 }) % 3;
            fields[focused].selected = true;
        }
        for (index, field) in fields.iter_mut().enumerate() {
            let top = point(150.0, 192.0 + index as f32 * 78.0);
            let rect = Rect::new(top.x, top.y, 500.0 * scale, 38.0 * scale);
            if is_mouse_button_pressed(MouseButton::Left) && rect.contains(mouse) {
                focused = index;
                field.cursor = field.value.len();
                field.selected = false;
            }
        }
        edit_field(&mut fields[focused]);

        clear_background(Color::from_rgba(10, 13, 21, 255));
        let accent = Color::from_rgba(0, 217, 244, 255);
        draw_rectangle(
            point(120.0, 140.0).x,
            point(120.0, 140.0).y,
            560.0 * scale,
            367.0 * scale,
            Color::from_rgba(19, 26, 40, 255),
        );
        draw_rectangle_lines(
            point(120.0, 140.0).x,
            point(120.0, 140.0).y,
            560.0 * scale,
            367.0 * scale,
            scale,
            Color::from_rgba(40, 72, 91, 255),
        );
        centered_text("PONG ONLINE", point(400.0, 92.0), 48.0 * scale, accent);
        centered_text(
            "Dos jugadores. Un servidor. La misma partida.",
            point(400.0, 120.0),
            18.0 * scale,
            Color::from_rgba(164, 184, 204, 255),
        );

        for (index, label) in [
            "Servidor (IP o nombre y puerto)",
            "Nickname",
            "Correo electrónico",
        ]
        .iter()
        .enumerate()
        {
            let y = 178.0 + index as f32 * 78.0;
            draw_text(
                label,
                point(150.0, y).x,
                point(150.0, y).y,
                18.0 * scale,
                Color::from_rgba(177, 194, 214, 255),
            );
            let top = point(150.0, y + 14.0);
            let rect = Rect::new(top.x, top.y, 500.0 * scale, 38.0 * scale);
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::from_rgba(9, 14, 24, 255),
            );
            draw_rectangle_lines(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                scale,
                if focused == index {
                    accent
                } else {
                    Color::from_rgba(53, 65, 83, 255)
                },
            );
            draw_field(&fields[index], rect, 21.0 * scale, focused == index, scale);
        }

        let button_pos = point(150.0, 432.0);
        let button = Rect::new(button_pos.x, button_pos.y, 500.0 * scale, 43.0 * scale);
        let hovered = button.contains(mouse);
        draw_rectangle(
            button.x,
            button.y,
            button.w,
            button.h,
            if hovered {
                Color::from_rgba(100, 241, 255, 255)
            } else {
                accent
            },
        );
        centered_text(
            "CONECTAR",
            point(400.0, 461.0),
            24.0 * scale,
            Color::from_rgba(6, 20, 28, 255),
        );
        connect |= hovered && is_mouse_button_pressed(MouseButton::Left);

        if connect {
            match NetworkConfig::new(
                fields[0].value.clone(),
                fields[1].value.clone(),
                fields[2].value.clone(),
            ) {
                Ok(config) => return Some(config),
                Err(message) => error = Some(message),
            }
        }
        if let Some(message) = &error {
            draw_form_error(message, point(150.0, 526.0), 500.0 * scale, 16.0 * scale);
        }
        centered_text(
            "TAB: Cambiar campo   |   ENTER: Conectar   |   ESC: Salir",
            point(400.0, 581.0),
            14.0 * scale,
            Color::from_rgba(133, 151, 173, 255),
        );
        next_frame().await;
    }
}

fn centered_text(text: &str, center: Vec2, size: f32, color: Color) {
    let width = measure_text(text, None, size as u16, 1.0).width;
    draw_text(text, center.x - width * 0.5, center.y, size, color);
}

fn draw_form_error(text: &str, origin: Vec2, max_width: f32, font_size: f32) {
    let mut line = String::new();
    let mut y = origin.y;
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.into()
        } else {
            format!("{line} {word}")
        };
        if measure_text(&candidate, None, font_size as u16, 1.0).width > max_width
            && !line.is_empty()
        {
            draw_text(
                &line,
                origin.x,
                y,
                font_size,
                Color::from_rgba(255, 153, 138, 255),
            );
            y += font_size * 1.25;
            line = word.into();
        } else {
            line = candidate;
        }
    }
    draw_text(
        &line,
        origin.x,
        y,
        font_size,
        Color::from_rgba(255, 153, 138, 255),
    );
}

fn draw_field(field: &TextField, rect: Rect, size: f32, focused: bool, scale: f32) {
    let max_width = rect.w - 20.0 * scale;
    let mut start = 0;
    while measure_text(&field.value[start..field.cursor], None, size as u16, 1.0).width > max_width
    {
        start += field.value[start..]
            .chars()
            .next()
            .map_or(0, char::len_utf8);
    }
    let mut end = field.cursor;
    while let Some(next) = field.value[end..].chars().next() {
        let candidate_end = end + next.len_utf8();
        if measure_text(&field.value[start..candidate_end], None, size as u16, 1.0).width
            > max_width
        {
            break;
        }
        end = candidate_end;
    }
    let position = vec2(rect.x + 9.0 * scale, rect.y + rect.h * 0.7);
    if focused && field.selected {
        draw_rectangle(
            position.x,
            rect.y + 4.0 * scale,
            measure_text(&field.value[start..end], None, size as u16, 1.0).width,
            rect.h - 8.0 * scale,
            Color::from_rgba(22, 79, 105, 255),
        );
    }
    draw_text(
        &field.value[start..end],
        position.x,
        position.y,
        size,
        WHITE,
    );
    if focused && get_time() % 1.0 < 0.55 {
        let caret = position.x
            + measure_text(&field.value[start..field.cursor], None, size as u16, 1.0).width;
        draw_line(
            caret,
            rect.y + 7.0 * scale,
            caret,
            rect.y + rect.h - 7.0 * scale,
            scale,
            WHITE,
        );
    }
}

#[derive(Default)]
struct TextField {
    value: String,
    cursor: usize,
    selected: bool,
}
impl TextField {
    fn new(value: &str) -> Self {
        Self {
            value: value.into(),
            cursor: value.len(),
            selected: false,
        }
    }
    fn insert(&mut self, text: &str) {
        if self.selected {
            self.value.clear();
            self.cursor = 0;
            self.selected = false;
        }
        for character in text.chars().filter(|character| !character.is_control()) {
            if self.value.len() + character.len_utf8() > 512 {
                break;
            }
            self.value.insert(self.cursor, character);
            self.cursor += character.len_utf8();
        }
    }
    fn left(&self) -> usize {
        self.value[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(index, _)| index)
    }
    fn backspace(&mut self) {
        if self.selected {
            self.value.clear();
            self.cursor = 0;
            self.selected = false;
        } else {
            let left = self.left();
            self.value.drain(left..self.cursor);
            self.cursor = left;
        }
    }
    fn delete(&mut self) {
        if self.selected {
            self.backspace();
        } else if let Some(character) = self.value[self.cursor..].chars().next() {
            self.value
                .drain(self.cursor..self.cursor + character.len_utf8());
        }
    }
}

fn edit_field(field: &mut TextField) {
    let control = is_key_down(KeyCode::LeftControl)
        || is_key_down(KeyCode::RightControl)
        || is_key_down(KeyCode::LeftSuper)
        || is_key_down(KeyCode::RightSuper);
    if control && is_key_pressed(KeyCode::A) {
        field.selected = true;
    }
    if control && is_key_pressed(KeyCode::C) && field.selected {
        macroquad::miniquad::window::clipboard_set(&field.value);
    }
    if control
        && is_key_pressed(KeyCode::V)
        && let Some(paste) = macroquad::miniquad::window::clipboard_get()
    {
        field.insert(&paste);
    }
    if is_key_pressed(KeyCode::Backspace) {
        field.backspace();
    }
    if is_key_pressed(KeyCode::Delete) {
        field.delete();
    }
    if is_key_pressed(KeyCode::Left) {
        field.cursor = if field.selected { 0 } else { field.left() };
        field.selected = false;
    }
    if is_key_pressed(KeyCode::Right) {
        field.cursor = if field.selected {
            field.value.len()
        } else {
            field.cursor
                + field.value[field.cursor..]
                    .chars()
                    .next()
                    .map_or(0, char::len_utf8)
        };
        field.selected = false;
    }
    if is_key_pressed(KeyCode::Home) {
        field.cursor = 0;
        field.selected = false;
    }
    if is_key_pressed(KeyCode::End) {
        field.cursor = field.value.len();
        field.selected = false;
    }
    while let Some(character) = get_char_pressed() {
        if !control {
            field.insert(&character.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_selects_form_online_or_explicit_demo() {
        assert!(matches!(parse_client_args(&[]).unwrap(), ClientMode::Form));
        assert!(matches!(
            parse_client_args(&["--demo".into()]).unwrap(),
            ClientMode::Demo
        ));
        assert!(matches!(
            parse_client_args(&["--help".into()]).unwrap(),
            ClientMode::Help
        ));
        let args = [
            "127.0.0.1:8080".into(),
            "José".into(),
            "jose@example.com".into(),
        ];
        match parse_client_args(&args).unwrap() {
            ClientMode::Online(config) => {
                assert_eq!(config.nickname, "José");
                assert_eq!(config.server_addr, "127.0.0.1:8080");
            }
            _ => panic!("se esperaba conexión online"),
        }
        assert!(parse_client_args(&["127.0.0.1:8080".into()]).is_err());
    }

    #[test]
    fn utf8_editing_maintains_cursor_boundaries() {
        let mut field = TextField::new("José");
        field.backspace();
        assert_eq!(field.value, "Jos");
        field.insert("é");
        field.cursor = field.left();
        field.delete();
        assert_eq!(field.value, "Jos");
        field.selected = true;
        field.insert("李\nAna");
        assert_eq!(field.value, "李Ana");
        assert_eq!(field.cursor, field.value.len());
    }
}
