//! Text input handling for SSH connection prompt fields.

pub fn handle_char(
    c: char,
    idx: usize,
    name: &mut String,
    host: &mut String,
    port: &mut String,
    user: &mut String,
    pass: &mut String,
    key_path: &mut String,
) {
    match idx {
        1 => name.push(c),
        2 => host.push(c),
        3 => {
            if c.is_ascii_digit() {
                port.push(c);
            }
        }
        4 => user.push(c),
        5 => pass.push(c),
        6 => key_path.push(c),
        _ => {}
    }
}

pub fn handle_backspace(
    idx: usize,
    name: &mut String,
    host: &mut String,
    port: &mut String,
    user: &mut String,
    pass: &mut String,
    key_path: &mut String,
) {
    match idx {
        1 => {
            name.pop();
        }
        2 => {
            host.pop();
        }
        3 => {
            port.pop();
        }
        4 => {
            user.pop();
        }
        5 => {
            pass.pop();
        }
        6 => {
            key_path.pop();
        }
        _ => {}
    }
}
