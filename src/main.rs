use std::collections::HashMap;
use std::fs::{self};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::path::PathBuf;
use regex::Regex;
use evalexpr::eval;
use crossterm::{
    cursor,
    event::{
        self,
        DisableMouseCapture,
        EnableMouseCapture,
        Event,
        KeyCode,
        KeyEvent,
        KeyModifiers,
        MouseButton,
        MouseEvent,
        MouseEventKind,
        KeyEventKind,
    },
    execute,
    queue,
    style::{Attribute, SetAttribute, Color, Print, SetForegroundColor, ResetColor},
    terminal::{
        disable_raw_mode,
        enable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen,
        Clear,
        ClearType,
        size,
    },
};
use clap::Parser;
use std::env;
use toml::Value;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    filename: Option<String>,
}

fn handle_func_command(
    command: &str,
    inputs: &mut Vec<String>,
    func_map: &mut HashMap<String, HashMap<String, String>>,
    const_map: &HashMap<String, String>,
    current_row: usize,
    func_toml_path: &Path
) -> bool {
    if command.starts_with("fc.") {
        let key = &command[3..];
        // 重新加载 .func.toml 文件
        if let Ok((new_func_map, _new_const_map, _custom_color, _custom_attribute)) = load_func_commands_from_file(func_toml_path) {
            *func_map = new_func_map;
        } else {
            eprintln!("Failed to reload .func.toml");
            return false;
        }

        if let Some(commands) = func_map.get(key) {
            for input in inputs.iter_mut().take(13) {
                input.clear();
            }
            for (input_key, input_value) in commands {
                let index = match input_key.as_str() {
                    "A" => 0,
                    "B" => 1,
                    "C" => 2,
                    "D" => 3,
                    "E" => 4,
                    "F" => 5,
                    "G" => 6,
                    "H" => 7,
                    "I" => 8,
                    "J" => 9,
                    "K" => 10,
                    "L" => 11,
                    "M" => 12,
                    "N" => 13,
                    _ => {
                        continue;
                    }
                };
                inputs[index] = input_value.to_string();
            }
            return true;
        }
    } else if command.starts_with("cst.") {
        let key = &command[4..];
        if let Some(value) = const_map.get(key) {
            inputs[current_row] = value.to_string();
            return true;
        }
    }
    false
}

fn load_func_commands_from_file(
    filename: &Path
) -> Result<(HashMap<String, HashMap<String, String>>, HashMap<String, String>, Option<String>, Option<String>), io::Error> {
    if !filename.exists() {
        let initial_content =
            r#"
[0]
A = ""
B = ""
C = ""
D = ""
E = ""
F = ""
G = ""
H = ""
I = ""
J = ""
K = ""
L = ""
M = ""
N = ""

[remarks]
R0 = ""

[const]
k = "1000.0 # Thousand"

[TUI]
color = "Green"
attribute = "Underlined"
"#;
        fs::write(filename, initial_content)?;
    }

    let content = fs::read_to_string(filename)?;
    let value: Value = toml::from_str(&content)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "- The configuration file .func.toml has a syntax error!\n\n- Please locate it in the working directory and check it,\n- or you can delete it to restore the factory settings.\n"))?;

    let mut func_map = HashMap::new();
    let mut const_map = HashMap::new();
    let mut custom_color = None;
    let mut custom_attribute = None;

    if let Value::Table(table) = value {
        for (key, value) in table {
            if key == "const" {
                if let Value::Table(const_table) = value {
                    for (const_key, const_value) in const_table {
                        if let Value::String(const_string) = const_value {
                            const_map.insert(const_key, const_string);
                        }
                    }
                }
            } else if key == "TUI" {
                if let Value::Table(tui_table) = value {
                    custom_color = tui_table.get("color").and_then(|v| v.as_str().map(String::from));
                    custom_attribute = tui_table.get("attribute").and_then(|v| v.as_str().map(String::from));
                }
            } else if let Value::Table(command_table) = value {
                let mut commands = HashMap::new();
                for (command_key, command_value) in command_table {
                    if let Value::String(command_string) = command_value {
                        commands.insert(command_key, command_string);
                    }
                }
                func_map.insert(key.to_lowercase(), commands);
            }
        }
    } else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "TOML root is not a table"));
    }

    Ok((func_map, const_map, custom_color, custom_attribute))
}

fn main() -> io::Result<()> {
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().unwrap();
    let func_toml_path = exe_dir.join(".func.toml");

    let (mut func_map, const_map, custom_color, custom_attribute) = match load_func_commands_from_file(&func_toml_path) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("{}", e);
            wait_for_keypress("- Press any key to exit ...");
            return Err(e);
        }
    };

    let args = Args::parse();
    let filename = args.filename.map(PathBuf::from).unwrap_or_else(|| exe_dir.join(".func.toml"));
    let (mut inputs, additional_lines) = read_inputs_from_file(&filename).unwrap_or_else(|_| (
        vec!["".to_string(); 14],
        vec![],
    ));
    let lock_state = Arc::new(RwLock::new(false));
    let current_section = Arc::new(RwLock::new("0".to_string()));

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    let result = run_app(
        &filename,
        &mut inputs,
        &additional_lines,
        Arc::clone(&lock_state),
        Arc::clone(&current_section),
        &mut func_map,
        &const_map,
        custom_color,
        custom_attribute,
        &func_toml_path
    );
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    result
}

fn wait_for_keypress(message: &str) {
    use crossterm::event::{read, Event, KeyCode};

    println!("{}", message);

    loop {
        if let Ok(Event::Key(key_event)) = read() {
            if let KeyCode::Char(_) | KeyCode::Enter | KeyCode::Esc = key_event.code {
                break;
            }
        }
    }
}

fn run_app(
    filename: &Path,
    inputs: &mut Vec<String>,
    additional_lines: &Vec<String>,
    lock_state: Arc<RwLock<bool>>,
    current_section: Arc<RwLock<String>>,
    func_map: &mut HashMap<String, HashMap<String, String>>,
    const_map: &HashMap<String, String>,
    custom_color: Option<String>,
    custom_attribute: Option<String>,
    func_toml_path: &Path
) -> io::Result<()> {
    let mut stdout = io::stdout();
    let mut variables = HashMap::new();
    let mut current_row = 0;
    let mut current_pos = 0;
    let input_width = 47;
    let output_width = 23;
    let title = " RS Mathematical Tools                                                   V1.2.5 ";
    let heade = "                     Result  =  Mathematical Expression               ";
    let foote = " About | Rate | fc.section | cst.item                  https://github.com/pasdq ";
    let saved = "                                Recalculate & Save to";
    let mut show_saved_message = false;
    let default_color = custom_color.unwrap_or_else(|| "Green".to_string());
    let default_attribute = custom_attribute.unwrap_or_else(|| "Underlined".to_string());

    let tui_color = match default_color.as_str() {
        "Blue" => Color::Blue,
        "Red" => Color::Red,
        "Green" => Color::Green,
        "Yellow" => Color::Yellow,
        "Magenta" => Color::Magenta,
        "Cyan" => Color::Cyan,
        "White" => Color::White,
        "Black" => Color::Black,
        "DarkRed" => Color::DarkRed,
        "DarkGreen" => Color::DarkGreen,
        "DarkYellow" => Color::DarkYellow,
        "DarkBlue" => Color::DarkBlue,
        "DarkMagenta" => Color::DarkMagenta,
        "DarkCyan" => Color::DarkCyan,
        "Grey" => Color::Grey,
        "DarkGrey" => Color::DarkGrey,
        _ => Color::Green, // Fallback color
    };

    let tui_attribute = match default_attribute.as_str() {
        "Bold" => Attribute::Bold,
        "Underlined" => Attribute::Underlined,
        "Reverse" => Attribute::Reverse,
        "NoBold" => Attribute::NoBold,
        "NoUnderline" => Attribute::NoUnderline,
        "NoReverse" => Attribute::NoReverse,
        "Italic" => Attribute::Italic,
        "NoItalic" => Attribute::NoItalic,
        "Dim" => Attribute::Dim,
        "NormalIntensity" => Attribute::NormalIntensity,
        "SlowBlink" => Attribute::SlowBlink,
        "RapidBlink" => Attribute::RapidBlink,
        "NoBlink" => Attribute::NoBlink,
        "Hidden" => Attribute::Hidden,
        "NoHidden" => Attribute::NoHidden,
        "CrossedOut" => Attribute::CrossedOut,
        "NotCrossedOut" => Attribute::NotCrossedOut,
        _ => Attribute::Underlined, // Fallback attribute
    };

    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    
    queue!(
        stdout,
        Clear(ClearType::All),
        SetAttribute(Attribute::Reverse),
        cursor::MoveTo(0, 0),
        Print(title),
        ResetColor
    )?;
    
    let result = loop {
        let is_locked = *lock_state.read().unwrap();
        let current_section_name = current_section.read().unwrap().clone();
        let (term_width, _) = size()?;

        let mut buffer = Vec::new();
        let section_length = current_section_name.len() as u16;
        queue!(
            buffer,
            SetAttribute(Attribute::Bold),
            SetForegroundColor(Color::Blue),
            cursor::MoveTo(0, 2),
            Print(heade),
	    cursor::MoveLeft(section_length + 1),
            Print(format!("Section: [{}]", current_section_name)),
            ResetColor
        )?;

        let mut results = Vec::new();
        for (i, input) in inputs.iter().enumerate() {
            let label = (b'A' + (i as u8)) as char;
            let result = if input.trim().is_empty() {
                "".to_string()
            } else {
                match evaluate_and_solve(input, &variables) {
                    Ok(res) => {
                        if res.len() <= output_width - 3 { res } else { "Error".to_string() }
                    }
                    Err(_) => {
                        if input.starts_with("fc.") {
                            "Import from cfg file".to_string()
                        } else {
                            "Error".to_string()
                        }
                    }
                }
            };
            if i == current_row {
                if result == "Error" || (input.starts_with("fc.") && result.is_empty()) {
                    queue!(
                        buffer,
                        SetForegroundColor(
                            if input.starts_with("fc.") {
                                Color::Blue
                            } else {
                                Color::DarkRed
                            }
                        ),
                        cursor::MoveTo(0, (i + 3) as u16),
                        Print(
                            format!(
                                "{}: [{:>width$}] = [{:<input_width$}]",
                                label,
                                if input.starts_with("fc.") {
                                    ""
                                } else {
                                    result.as_str()
                                },
                                input,
                                width = output_width,
                                input_width = input_width
                            )
                        ),
                        ResetColor
                    )?;
                } else {
                    if !is_locked {
                        queue!(
                            buffer,
                            SetForegroundColor(tui_color),
                            SetAttribute(tui_attribute),
                            cursor::MoveTo(0, (i + 3) as u16),
                            Print(
                                format!(
                                    "{}: [{:>width$}] = [{:<input_width$}]",
                                    label,
                                    result,
                                    input,
                                    width = output_width,
                                    input_width = input_width
                                )
                            ),
                            ResetColor
                        )?;
                    } else {
                        queue!(
                            buffer,
                            SetForegroundColor(if i >= 12 { Color::Blue } else { tui_color }),
                            cursor::MoveTo(0, (i + 3) as u16),
                            Print(
                                format!(
                                    "{}: [{:>width$}] = [{:<input_width$}]",
                                    label,
                                    result,
                                    input,
                                    width = output_width,
                                    input_width = input_width
                                )
                            ),
                            ResetColor
                        )?;
                    }
                }
            } else {
                if result == "Error" || (input.starts_with("fc.") && result.is_empty()) {
                    queue!(
                        buffer,
                        SetForegroundColor(
                            if input.starts_with("fc.") {
                                Color::Blue
                            } else {
                                Color::DarkRed
                            }
                        ),
                        cursor::MoveTo(0, (i + 3) as u16),
                        Print(
                            format!(
                                "{}: [{:>width$}] = [{:<input_width$}]",
                                label,
                                if input.starts_with("fc.") {
                                    ""
                                } else {
                                    result.as_str()
                                },
                                input,
                                width = output_width,
                                input_width = input_width
                            )
                        ),
                        ResetColor
                    )?;
                } else {
                    queue!(
                        buffer,
                        SetForegroundColor(if i >= 12 { Color::Blue } else { Color::Reset }),
                        cursor::MoveTo(0, (i + 3) as u16),
                        Print(
                            format!(
                                "{}: [{:>width$}] = [{:<input_width$}]",
                                label,
                                result,
                                input,
                                width = output_width,
                                input_width = input_width
                            )
                        ),
                        ResetColor
                    )?;
                }
            }
            if result != "Error" {
                variables.insert(label.to_string(), result.clone());
            } else {
                variables.remove(&label.to_string());
            }
            results.push(result);
        }
        let (sum, valid_count) = calculate_sum_and_count(&results);
        let average = if valid_count > 0 { sum / (valid_count as f64) } else { 0.0 };
        queue!(
            buffer,
            cursor::MoveTo(0, (inputs.len() + 4) as u16),
            Print(" ".repeat(term_width as usize)),
            cursor::MoveTo(0, (inputs.len() + 5) as u16),
            Print(" ".repeat(term_width as usize)),
            SetForegroundColor(Color::Blue),
            cursor::MoveTo(17, (inputs.len() + 4) as u16),
            Print(format!("(A - L) Sum = {}", format_with_thousands_separator(sum))),
            cursor::MoveTo(13, (inputs.len() + 5) as u16),
            Print(format!("(A - L) Average = {}", format_with_thousands_separator(average))),
            cursor::MoveTo(0, (inputs.len() + 8) as u16),
            ResetColor,
            SetAttribute(Attribute::Reverse),
            Print(foote),
            ResetColor,
            cursor::MoveTo(22, 20),
            SetForegroundColor(if is_locked { Color::Red } else { Color::Green }),
            Print(
                format!("Status = {} (F4 Status Switch)", if is_locked {
                    "Locked"
                } else {
                    "Opened"
                })
            ),
            ResetColor,
            cursor::MoveTo(0, (inputs.len() + 9) as u16),
            ResetColor
        )?;
        
        if show_saved_message {
            queue!(
                buffer,
                cursor::MoveTo(0, 17),
                SetForegroundColor(Color::DarkYellow),
		Print(format!("{} -> Section: [{}]", saved, current_section_name)),
                ResetColor
            )?;
            show_saved_message = false;
        } else {
            queue!(buffer, cursor::MoveTo(0, 17), Print(" ".repeat(term_width as usize)))?;
        }
        
        for (i, line) in additional_lines.iter().enumerate() {
            queue!(buffer, cursor::MoveTo(0, (inputs.len() + 10 + i) as u16), Print(line))?;
        }

        if is_locked {
            queue!(buffer, cursor::Hide)?;
        } else {
            let cursor_position = (output_width + 9 + current_pos) as u16;
            queue!(
                buffer,
                cursor::MoveTo(cursor_position, (current_row + 3) as u16),
                cursor::Show
            )?;
        }
        
        stdout.write_all(&buffer)?;
        stdout.flush()?;
        
        match event::read()? {
            Event::Key(KeyEvent { code, modifiers, kind, .. }) =>
                match (code, kind) {
                    (KeyCode::Char('c'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        break;
                    }
                    (KeyCode::PageUp, KeyEventKind::Press) => {
                        handle_page_up(current_section.clone(), func_map, inputs, func_toml_path);
                    }
                    
                    (KeyCode::PageDown, KeyEventKind::Press) => {
                        handle_page_down(current_section.clone(), func_map, inputs, func_toml_path);
                    }
		    (KeyCode::F(4), KeyEventKind::Press) => {
                        let mut lock_state_guard = lock_state.write().unwrap();
                        *lock_state_guard = !*lock_state_guard;
                        if *lock_state_guard {
                            queue!(stdout, cursor::Hide)?;
                        } else {
                            queue!(stdout, cursor::Show)?;
                        }
                    }
			(KeyCode::Char('u'), KeyEventKind::Press) if modifiers.contains(KeyModifiers::CONTROL) => {
                        if !is_locked {
                            for input in inputs.iter_mut().take(13) {
                                input.clear();
                            }
                            for label in (b'A'..=b'L').map(|c| (c as char).to_string()) {
                                variables.remove(&label);
                            }
                            current_pos = 0;
                            current_row = 0;
                        }
                    }
                    (KeyCode::Char('l'), KeyEventKind::Press) if modifiers.contains(KeyModifiers::CONTROL) => {
                        if !is_locked {
                            let label = (b'A' + (current_row as u8)) as char;
                            inputs[current_row].clear();
                            variables.remove(&label.to_string());
                            current_pos = 0;
                        }
                    }
                    (KeyCode::F(5), key_event_kind) if (cfg!(target_os = "windows") && key_event_kind == KeyEventKind::Release) || (cfg!(target_os = "linux") && key_event_kind == KeyEventKind::Press) => {
                        save_inputs_to_file(filename, inputs, additional_lines, &current_section.read().unwrap())?;
                        show_saved_message = true;
                        queue!(buffer, Clear(ClearType::All), cursor::MoveTo(0, 0), Print(title))?;
                        variables.clear();
                        for (i, input) in inputs.iter().enumerate() {
                            let label = (b'A' + (i as u8)) as char;
                            if !input.trim().is_empty() {
                                match evaluate_and_solve(input, &variables) {
                                    Ok(res) => {
                                        variables.insert(label.to_string(), res);
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                    }
                    (KeyCode::Char('t'), KeyEventKind::Press) if modifiers.contains(KeyModifiers::CONTROL) => {
                        if !is_locked {
                            current_row = 0;
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::Char('a'), KeyEventKind::Press) if modifiers.contains(KeyModifiers::CONTROL) => {
                        if !is_locked {
                            current_pos = 0;
                        }
                    }
                    (KeyCode::Char('b'), KeyEventKind::Press) if modifiers.contains(KeyModifiers::CONTROL) => {
                        if !is_locked {
                            current_row = inputs.len() - 1;
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::Char('e'), KeyEventKind::Press) if modifiers.contains(KeyModifiers::CONTROL) => {
                        if !is_locked {
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::Down | KeyCode::Tab, KeyEventKind::Press) => {
                        if !is_locked && current_row + 1 < inputs.len() {
                            current_row += 1;
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::Up, KeyEventKind::Press) => {
                        if !is_locked && current_row > 0 {
                            current_row -= 1;
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::Left, KeyEventKind::Press) => {
                        if !is_locked && current_pos > 0 {
                            current_pos -= 1;
                        }
                    }
                    (KeyCode::Right, KeyEventKind::Press) => {
                        if !is_locked && current_pos < inputs[current_row].len() {
                            current_pos += 1;
                        }
                    }
                    (KeyCode::Backspace, KeyEventKind::Press) => {
                        if !is_locked && current_pos > 0 {
                            inputs[current_row].remove(current_pos - 1);
                            current_pos -= 1;
                        }
                    }
                    (KeyCode::Enter, KeyEventKind::Press) => {
            if !is_locked {
                let input_command = inputs[current_row].clone().to_lowercase();
                if input_command.starts_with("fc.") && handle_func_command(&input_command, inputs, func_map, const_map, current_row, func_toml_path) {
                    current_pos = inputs[current_row].len();
                    *current_section.write().unwrap() = input_command[3..].to_string();
                } else if input_command.starts_with("cst.") && handle_func_command(&input_command, inputs, func_map, const_map, current_row, func_toml_path) {
                    current_pos = inputs[current_row].len();
                } else if input_command == "about" {
                    inputs[current_row].clear();
                    inputs[current_row].push_str("# RS Mathematical Tools V1.2.5");
                    current_pos = inputs[current_row].len();
                } else if input_command == "rate" {
                    inputs[current_row].clear();
                    let exe_path = env::current_exe().unwrap();
                    let exe_dir = exe_path.parent().unwrap();
                    let command_path = if cfg!(target_os = "windows") {
                        exe_dir.join("rate.exe")
                    } else {
                        exe_dir.join("./rate")
                    };
                    let output = std::process::Command::new(command_path).output();
                    match output {
                        Ok(output) => {
                            let result = String::from_utf8_lossy(&output.stdout);
                            let trimmed_result = result.trim();
                            inputs[current_row].push_str(trimmed_result);
                        }
                        Err(_) => {
                            inputs[current_row].push_str("The rate command was not found!");
                        }
                    }
                    current_pos = inputs[current_row].len();
                } else {
                    current_row = (current_row + 1) % inputs.len();
                    current_pos = inputs[current_row].len();
                }
            }
        }
                    (KeyCode::Char(c), KeyEventKind::Press) if !is_locked && c.is_ascii() => {
                        if inputs[current_row].len() < input_width {
                            inputs[current_row].insert(current_pos, c);
                            current_pos += 1;
                        }
                    }
                    _ => {}
                }
            Event::Mouse(MouseEvent { kind, column, row, .. }) =>
                match kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        let clicked_row = (row as usize) - 3;
                        if !is_locked && (3..inputs.len() + 3).contains(&(row as usize)) {
                            current_row = clicked_row;
                            current_pos = (column as usize) - (output_width + 9);
                            if current_pos > inputs[current_row].len() {
                                current_pos = inputs[current_row].len();
                            }
                            let label = (b'A' + (current_row as u8)) as char;
                            let result = if inputs[current_row].trim().is_empty() {
                                "".to_string()
                            } else {
                                match evaluate_and_solve(&inputs[current_row], &variables) {
                                    Ok(res) => {
                                        if res.len() <= output_width - 3 {
                                            res
                                        } else {
                                            "Error".to_string()
                                        }
                                    }
                                    Err(_) => "Error".to_string(),
                                }
                            };
                            queue!(
                                buffer,
                                cursor::MoveTo(0, (current_row + 3) as u16),
                                Print(
                                    format!(
                                        "{}: [{:>width$}] = [{:<input_width$}]",
                                        label,
                                        result,
                                        inputs[current_row],
                                        width = output_width,
                                        input_width = input_width
                                    )
                                )
                            )?;
                            stdout.write_all(&buffer)?;
                            stdout.flush()?;
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if !is_locked && current_row + 1 < inputs.len() {
                            current_row += 1;
                            current_pos = inputs[current_row].len();
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        if !is_locked && current_row > 0 {
                            current_row -= 1;
                            current_pos = inputs[current_row].len();
                        }
                    }
                    _ => {}
                }
            _ => {}
        }
    };
    Ok(result)
}

/// 从指定文件读取输入数据
/// 如果文件不存在或为空, 则初始化输入列表和附加行
fn read_inputs_from_file(filename: &Path) -> Result<(Vec<String>, Vec<String>), io::Error> {
    if !filename.exists() || fs::metadata(filename)?.len() == 0 {
        let initial_content =
            r#"
[0]
A = ""
B = ""
C = ""
D = ""
E = ""
F = ""
G = ""
H = ""
I = ""
J = ""
K = ""
L = ""
M = ""
N = ""

[remarks]
R0 = ""
"#;
        fs::write(filename, initial_content)?;
        return Ok((vec!["".to_string(); 14], vec![]));
    }
    
    let mut content = fs::read_to_string(filename)?;
    let mut value: Value = toml::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    if let Value::Table(ref mut table) = value {
        if !table.contains_key("0") {
            let mut initial_0_section = toml::map::Map::new();
            initial_0_section.insert("A".to_string(), Value::String("".to_string()));
            initial_0_section.insert("B".to_string(), Value::String("".to_string()));
            initial_0_section.insert("C".to_string(), Value::String("".to_string()));
            initial_0_section.insert("D".to_string(), Value::String("".to_string()));
            initial_0_section.insert("E".to_string(), Value::String("".to_string()));
            initial_0_section.insert("F".to_string(), Value::String("".to_string()));
            initial_0_section.insert("G".to_string(), Value::String("".to_string()));
                        initial_0_section.insert("H".to_string(), Value::String("".to_string()));
            initial_0_section.insert("I".to_string(), Value::String("".to_string()));
            initial_0_section.insert("J".to_string(), Value::String("".to_string()));
            initial_0_section.insert("K".to_string(), Value::String("".to_string()));
            initial_0_section.insert("L".to_string(), Value::String("".to_string()));
            initial_0_section.insert("M".to_string(), Value::String("".to_string()));
            initial_0_section.insert("N".to_string(), Value::String("".to_string()));

            table.insert("0".to_string(), Value::Table(initial_0_section));
            content = toml::to_string(&value).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            fs::write(filename, content.clone())?;
        }
    }

    let value: Value = toml::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    let mut inputs = vec!["".to_string(); 14];
    let mut additional_lines = vec![];

    if let Value::Table(table) = value {
        if let Some(Value::Table(input_table)) = table.get("0") {
            for (key, value) in input_table {
                if let Value::String(input_string) = value {
                    let index = match key.as_str() {
                        "A" => 0,
                        "B" => 1,
                        "C" => 2,
                        "D" => 3,
                        "E" => 4,
                        "F" => 5,
                        "G" => 6,
                        "H" => 7,
                        "I" => 8,
                        "J" => 9,
                        "K" => 10,
                        "L" => 11,
                        "M" => 12,
                        "N" => 13,
                        _ => {
                            continue;
                        }
                    };
                    inputs[index] = input_string.clone();
                }
            }
        }
        if let Some(Value::Table(remarks_table)) = table.get("remarks") {
            for (_key, value) in remarks_table {
                if let Value::String(remark_string) = value {
                    additional_lines.push(remark_string.clone());
                }
            }
        }
    }

    Ok((inputs, additional_lines))
}

/// 将当前输入数据和附加行的状态保存到指定文件
/// 将当前输入数据的状态保存到指定文件（不包括附加行）
fn save_inputs_to_file(
    filename: &Path,
    inputs: &[String],
    _additional_lines: &[String],
    section: &str
) -> Result<(), io::Error> {
    let mut value = if filename.exists() {
        let content = fs::read_to_string(filename)?;
        toml::from_str(&content).unwrap_or(Value::Table(toml::map::Map::new()))
    } else {
        Value::Table(toml::map::Map::new())
    };

    if let Value::Table(ref mut table) = value {
        // Preserve other sections and items in the table
        let mut input_table = table.get(section).cloned().unwrap_or(Value::Table(toml::map::Map::new()));
        if let Value::Table(ref mut input_table) = input_table {
            for (i, input) in inputs.iter().enumerate() {
                let label = (b'A' + (i as u8)) as char;
                if !input.is_empty() {
                    input_table.insert(label.to_string(), Value::String(input.clone()));
                } else {
                    input_table.remove(&label.to_string());
                }
            }
        }
        table.insert(section.to_string(), input_table);
    }

    let toml_string = toml::to_string(&value).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(filename, toml_string)?;
    Ok(())
}

/// 评估和求解输入中提供的数学表达式或方程
/// 支持线性方程和带变量替换的一般表达式
fn evaluate_and_solve(input: &str, variables: &HashMap<String, String>) -> Result<String, String> {
    if input.starts_with("fc.") {
        return Ok("Import from cfg file".to_string());
    }
    let input_without_comment = match input.find('#') {
        Some(pos) => &input[..pos],
        None => input,
    };
    let parts: Vec<&str> = input_without_comment.split('=').collect();
    if parts.len() == 2 {
        let lhs = parts[0].replace(" ", "").replace("X", "x");
        let rhs = parts[1].replace(" ", "").replace("X", "x");
        let lhs_replaced = replace_variables(lhs, variables);
        let rhs_replaced = replace_variables(rhs, variables);
        let lhs_replaced = lhs_replaced.replace("/", "*1.0/");
        let rhs_replaced = rhs_replaced.replace("/", "*1.0/");
        if lhs_replaced.contains('x') || rhs_replaced.contains('x') {
            let x = "x";
            let lhs_value = match eval(&replace_percentage(&lhs_replaced.replace(x, "0.0"))) {
                Ok(val) => val.as_number().unwrap_or(0.0),
                Err(_) => {
                    return Err("Error".to_string());
                }
            };
            let _rhs_value = match eval(&replace_percentage(&rhs_replaced.replace(x, "0.0"))) {
                Ok(val) => val.as_number().unwrap_or(0.0),
                Err(_) => {
                    return Err("Error".to_string());
                }
            };
            let coefficient = match eval(&replace_percentage(&lhs_replaced.replace(x, "1.0"))) {
                Ok(val) => val.as_number().unwrap_or(0.0) - lhs_value,
                Err(_) => {
                    return Err("Error".to_string());
                }
            };
            if coefficient == 0.0 {
                return Err(
                    "Invalid equation: coefficient of x is zero or not a linear equation".to_string()
                );
            }
            let result = (_rhs_value - lhs_value) / coefficient;
            let formatted_result = format_with_thousands_separator(result);
            Ok(formatted_result)
        } else {
            let lhs_value = match eval(&replace_percentage(&lhs_replaced)) {
                Ok(val) => val.as_number().unwrap_or(0.0),
                Err(_) => {
                    return Err("Error".to_string());
                }
            };
            let rhs_value = match eval(&replace_percentage(&rhs_replaced)) {
                Ok(val) => val.as_number().unwrap_or(0.0),
                Err(_) => {
                    return Err("Error".to_string());
                }
            };
            if lhs_value == rhs_value {
                Ok(format_with_thousands_separator(lhs_value))
            } else {
                Err("The equation is not balanced".to_string())
            }
        }
    } else if parts.len() == 1 {
        let expression = replace_variables(parts[0].replace(" ", ""), variables);
        let expression = expression.replace("/", "*1.0/");
        match eval(&replace_percentage(&expression)) {
            Ok(result) => {
                let formatted_result = format_with_thousands_separator(
                    result.as_number().unwrap_or(0.0)
                );
                Ok(formatted_result)
            }
            Err(_) => Err("Invalid mathematical expression.".to_string()),
        }
    } else {
        Err(
            "Invalid input format. Use a linear equation 'a*x + b = c' or a mathematical expression.".to_string()
        )
    }
}

/// 将数学表达式中的百分比（例如 `50%`）替换为其小数等价物（例如 `0.5`）
fn replace_percentage(expression: &str) -> String {
    let re = Regex::new(r"(\d+(\.\d+)?)%").unwrap();
    re.replace_all(expression, "$1 * 0.01").to_string()
}

/// 将数学表达式中的变量名替换为给定变量映射中的相应值
fn replace_variables(expression: String, variables: &HashMap<String, String>) -> String {
    let mut replaced_expression = expression.to_lowercase();
    let current_row = (b'A' + (variables.len() as u8)) as char;
    for (var, value) in variables {
        if var.to_lowercase() == expression.trim().to_lowercase() {
            if var.to_lowercase().chars().next().unwrap() == current_row {
                return "Error: Variable self-reference detected".to_string();
            }
        }
        let cleaned_value = remove_thousands_separator(value);
        let pattern = format!(r"\b{}\b", var.to_lowercase());
        let regex = Regex::new(&pattern).unwrap();
        replaced_expression = regex
            .replace_all(&replaced_expression, cleaned_value.as_str())
            .to_string();
    }
    replaced_expression
}

/// 计算结果列表中有效数值结果的总和和数量
/// 最多处理前 12 个结果
fn calculate_sum_and_count(results: &[String]) -> (f64, usize) {
    let mut sum = 0.0;
    let mut count = 0;
    for result in results.iter().take(12) {
        let cleaned_result = remove_thousands_separator(result);
        match cleaned_result.parse::<f64>() {
            Ok(val) => {
                sum += val;
		                count += 1;
            }
            Err(_) => {}
        }
    }
    (sum, count)
}

/// 格式化数值, 在数值中添加千位分隔符以提高可读性
fn format_with_thousands_separator(value: f64) -> String {
    let formatted_value = format!("{:.3}", value);
    let parts: Vec<&str> = formatted_value.split('.').collect();
    let int_part = parts[0];
    let dec_part = parts.get(1).unwrap_or(&"");

    let formatted_int = int_part
        .chars()
        .rev()
        .collect::<Vec<_>>()
        .chunks(3)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(",")
        .chars()
        .rev()
        .collect::<String>();

    if dec_part.is_empty() {
        formatted_int
    } else {
        format!("{}.{}", formatted_int, dec_part)
    }
}

/// 移除格式化数值中的千位分隔符以便进一步处理
fn remove_thousands_separator(value: &str) -> String {
    value.replace(",", "")
}

/// 循环切换 section
fn load_section(section: &str, inputs: &mut Vec<String>, func_toml_path: &Path) {
    if let Ok((func_map, _, _, _)) = load_func_commands_from_file(func_toml_path) {
        if let Some(commands) = func_map.get(section) {
            for input in inputs.iter_mut().take(13) {
                input.clear();
            }
            for (input_key, input_value) in commands {
                let index = match input_key.as_str() {
                    "A" => 0,
                    "B" => 1,
                    "C" => 2,
                    "D" => 3,
                    "E" => 4,
                    "F" => 5,
                    "G" => 6,
                    "H" => 7,
                    "I" => 8,
                    "J" => 9,
                    "K" => 10,
                    "L" => 11,
                    "M" => 12,
                    "N" => 13,
                    _ => continue,
                };
                inputs[index] = input_value.to_string();
            }
        }
    } else {
        eprintln!("Failed to reload .func.toml");
    }
}

/// 循环切换 section
fn get_next_section(func_map: &HashMap<String, HashMap<String, String>>, current_section: &str, reverse: bool) -> String {
    let mut keys: Vec<String> = func_map.keys().cloned().collect();
    keys.retain(|k| k != "tui" && k != "remarks" && k != "const");
    keys.sort();
    let current_index = keys.iter().position(|k| k == current_section).unwrap_or(0);

    if reverse {
        if current_index == 0 {
            keys[keys.len() - 1].clone()
        } else {
            keys[current_index - 1].clone()
        }
    } else {
        if current_index == keys.len() - 1 {
            keys[0].clone()
        } else {
            keys[current_index + 1].clone()
        }
    }
}

/// 循环切换 section
fn handle_page_up(current_section: Arc<RwLock<String>>, func_map: &mut HashMap<String, HashMap<String, String>>, inputs: &mut Vec<String>, func_toml_path: &Path) {
    let new_section = {
        let current_section_name = current_section.read().unwrap().clone();
        get_next_section(func_map, &current_section_name, true)
    };
    *current_section.write().unwrap() = new_section.clone();
    load_section(&new_section, inputs, func_toml_path);
}

/// 循环切换 section
fn handle_page_down(current_section: Arc<RwLock<String>>, func_map: &mut HashMap<String, HashMap<String, String>>, inputs: &mut Vec<String>, func_toml_path: &Path) {
    let new_section = {
        let current_section_name = current_section.read().unwrap().clone();
        get_next_section(func_map, &current_section_name, false)
    };
    *current_section.write().unwrap() = new_section.clone();
    load_section(&new_section, inputs, func_toml_path);
}
