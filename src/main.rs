use clap::Parser;
use crossterm::{
    cursor,
    event::{
        self,
        read,
        DisableMouseCapture,
        EnableMouseCapture,
        Event,
        KeyCode,
        KeyEvent,
        KeyEventKind,
        KeyModifiers,
        MouseButton,
        MouseEvent,
        MouseEventKind,
    },
    execute,
    queue,
    style::{ Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor },
    terminal::{
        disable_raw_mode,
        enable_raw_mode,
        size,
        Clear,
        ClearType,
        EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use evalexpr::eval;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{ self, Write };
use std::path::{ Path, PathBuf };
use std::sync::{ Arc, RwLock };
use toml::Value;

// 仅在向 windows 平台编译时才会导入这个模块
#[cfg(target_os = "windows")]
use arboard::Clipboard;

#[macro_use]
extern crate lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref GLOBAL_SUM: Mutex<f64> = Mutex::new(0.0);
    static ref GLOBAL_COUNT: Mutex<usize> = Mutex::new(0);
    static ref DECIMAL_PLACES: Mutex<usize> = Mutex::new(4); // 设置默认保留四位小数
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    filename: Option<String>,
}

struct TerminalManager;

impl TerminalManager {
    pub fn new() -> io::Result<Self> {
        let mut stdout = io::stdout();
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        Ok(Self)
    }
}

impl Drop for TerminalManager {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        disable_raw_mode().unwrap();
        execute!(stdout, LeaveAlternateScreen, DisableMouseCapture).unwrap();
    }
}

fn handle_fc_command(
    command: &str,
    inputs: &mut Vec<String>,
    func_map: &mut HashMap<String, HashMap<String, String>>,
    func_toml_path: &Path
) -> bool {
    let key = &command[3..];
    if let Ok((new_func_map, _, _, _, _)) = load_func_commands_from_file(func_toml_path) {
        *func_map = new_func_map;
    } else {
        eprintln!("Failed to reload .func.toml");
        return false;
    }

    if let Some(commands) = func_map.get(key) {
        for input in inputs.iter_mut().take(20) {
            // 修改此处为20
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
                "O" => 14,
                "P" => 15,
                "Q" => 16,
                "R" => 17,
                "S" => 18,
                "T" => 19,
                _ => {
                    continue;
                }
            };
            inputs[index] = input_value.to_string();
        }
        return true;
    }
    false
}

fn handle_const_command(
    command: &str,
    inputs: &mut Vec<String>,
    const_map: &HashMap<String, String>,
    current_row: usize
) -> bool {
    if let Some(value) = const_map.get(command) {
        inputs[current_row] = value.to_string();
        return true;
    }
    false
}

fn load_func_commands_from_file(
    filename: &Path
) -> Result<
    (
        HashMap<String, HashMap<String, String>>,
        HashMap<String, String>,
        Option<String>,
        Option<String>,
        f64,
    ),
    io::Error
> {
    if !filename.exists() {
        let initial_content =
            r#"
[home]
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
O = ""
P = ""
Q = ""
R = ""
S = ""
T = ""

[remarks]
R0 = ""

[const]
k = "1000.0 # Thousand"

[TUI]
color = "Green"
attribute = "Underlined"
step = "0.1"
"#;
        fs::write(filename, initial_content)?;
    }

    let content = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(err) => {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to read {}: {}", filename.display(), err)
                )
            );
        }
    };

    let value: Value = match toml::from_str(&content) {
        Ok(value) => value,
        Err(err) => {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid TOML in {}: {}", filename.display(), err)
                )
            );
        }
    };

    let mut func_map = HashMap::new();
    let mut const_map = HashMap::new();
    let mut custom_color = None;
    let mut custom_attribute = None;
    let mut step = 0.1;

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
                    custom_color = tui_table
                        .get("color")
                        .and_then(|v| v.as_str().map(String::from));
                    custom_attribute = tui_table
                        .get("attribute")
                        .and_then(|v| v.as_str().map(String::from));
                    if
                        let Some(step_value) = tui_table
                            .get("step")
                            .and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                    {
                        step = step_value;
                    }
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
        return Err(
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid TOML in {}: expected a table", filename.display())
            )
        );
    }

    Ok((func_map, const_map, custom_color, custom_attribute, step))
}

fn main() -> io::Result<()> {
    let _terminal_manager = TerminalManager::new()?;

    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().unwrap();
    let func_toml_path = exe_dir.join(".func.toml");
    let backup_toml_path = exe_dir.join(".func.toml.bak");

    if !func_toml_path.exists() {
        // 创建一个新的 .func.toml 文件之前，先创建一个备份文件
        let initial_content =
            r#"
[home]
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
O = ""
P = ""
Q = ""
R = ""
S = ""
T = ""

[remarks]
R0 = ""

[const]
k = "1000.0 # Thousand"

[TUI]
color = "Green"
attribute = "Underlined"
step = "0.1"
"#;
        fs::write(&func_toml_path, initial_content)?;
        fs::write(&backup_toml_path, initial_content)?;
    } else {
        // 如果 .func.toml 文件存在，创建其备份
        fs::copy(&func_toml_path, &backup_toml_path)?;
    }

    let (mut func_map, const_map, custom_color, custom_attribute, step) = match
        load_func_commands_from_file(&func_toml_path)
    {
        Ok(result) => result,
        Err(e) => {
            eprintln!("{}", e);
            wait_for_keypress("- Press any key to exit ...");
            return Err(e);
        }
    };

    let args = Args::parse();
    let filename = args.filename.map(PathBuf::from).unwrap_or_else(|| exe_dir.join(".func.toml"));
    let (mut inputs, additional_lines) = read_inputs_from_file(&filename).unwrap_or_else(|_| {
        (vec!["".to_string(); 20], vec![]) // 修改此处为20
    });
    let lock_state = Arc::new(RwLock::new(false));
    let current_section = Arc::new(RwLock::new("home".to_string()));

    let undo_stack = Arc::new(RwLock::new(Vec::new()));

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
        step, // 传递step值
        &func_toml_path,
        Arc::clone(&undo_stack)
    );
    result
}

fn wait_for_keypress(message: &str) {
    use crossterm::event::{ read, Event, KeyCode };

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
    step: f64,
    func_toml_path: &Path,
    undo_stack: Arc<RwLock<Vec<Vec<String>>>>
) -> io::Result<()> {
    let mut stdout = io::stdout();
    let mut variables = HashMap::new();
    let mut current_row = 0;
    let mut current_pos = 0;
    let input_width = 60;
    let output_width = 20;
    let title =
        " RS Mathematical Tools                                                             V1.2.9 ";
    let heade =
        "                  Result  =  Mathematical Expression                                  ";
    let foote =
        " About | Rate | Fc:Sec | Clear | New | Delete | Clone | Rename | Set     github.com/pasdq ";
    let saved = "                             Recalculate & Save to";
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
        _ => Color::Green,
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
        _ => Attribute::Underlined,
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
            cursor::MoveLeft(section_length + 2),
            Print(format!("<- {} ->", current_section_name)),
            ResetColor
        )?;

        let mut results = Vec::new();
        for (i, input) in inputs.iter().enumerate() {
            let label = (b'A' + (i as u8)) as char;
            let result = if input.trim().is_empty() {
                "".to_string()
            } else {
                match evaluate_and_solve(input, &variables, i) {
                    Ok(res) => {
                        if res.len() <= output_width - 3 { res } else { "Error".to_string() }
                    }
                    Err(_) => {
                        if input.starts_with("fc:") {
                            "Import from cfg file".to_string()
                        } else {
                            "Error".to_string()
                        }
                    }
                }
            };

            if i == current_row {
                if result == "Error" || (input.starts_with("fc:") && result.is_empty()) {
                    queue!(
                        buffer,
                        SetForegroundColor(
                            if input.starts_with("fc:") {
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
                                if input.starts_with("fc:") {
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
                            SetForegroundColor(if i >= 17 { Color::Blue } else { tui_color }), // 修改此处为17
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
                if result == "Error" || (input.starts_with("fc:") && result.is_empty()) {
                    queue!(
                        buffer,
                        SetForegroundColor(
                            if input.starts_with("fc:") {
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
                                if input.starts_with("fc:") {
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
                        SetForegroundColor(if i >= 17 { Color::Blue } else { Color::Reset }), // 修改此处为17
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
            cursor::MoveTo(10, (inputs.len() + 4) as u16),
            Print(format!("(A - Q) Sum = Z = {}", format_with_thousands_separator(sum))),
            cursor::MoveTo(10, (inputs.len() + 5) as u16),
            Print(format!("(A - Q) Average = {}", format_with_thousands_separator(average))),
            cursor::MoveTo(0, (inputs.len() + 8) as u16),
            ResetColor,
            SetAttribute(Attribute::Reverse),
            Print(foote),
            ResetColor,
            cursor::MoveTo(19, 26),
            SetForegroundColor(if is_locked { Color::Red } else { Color::Green }),
            Print(
                format!("Status = {} (F4 Status Switch & Align #)", if is_locked {
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
                cursor::MoveTo(0, 23),
                SetForegroundColor(Color::DarkYellow),
                Print(format!("{} -> Section: [{}]", saved, current_section_name)),
                ResetColor
            )?;
            show_saved_message = false;
        } else {
            queue!(buffer, cursor::MoveTo(0, 23), Print(" ".repeat(term_width as usize)))?;
        }

        for (i, line) in additional_lines.iter().enumerate() {
            queue!(buffer, cursor::MoveTo(0, (inputs.len() + 10 + i) as u16), Print(line))?;
        }

        if is_locked {
            queue!(buffer, cursor::Hide)?;
        } else {
            // 计算光标位置
            let cursor_position = (output_width + 9 + current_pos) as u16;
            queue!(
                buffer,
                cursor::MoveTo(cursor_position, (current_row + 3) as u16),
                cursor::Show
            )?;
        }

        stdout.write_all(&buffer)?;
        stdout.flush()?;

        let mut last_row = None;
        let mut last_pos = None;

        // 在循环中调用 display_current_position
        display_current_position(
            &mut stdout,
            current_row,
            current_pos,
            &mut last_row,
            &mut last_pos
        )?;

        match event::read()? {
            Event::Key(KeyEvent { code, modifiers, kind, .. }) =>
                match (code, kind) {
                    (KeyCode::Char('q'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        break;
                    }

                    (KeyCode::PageUp, KeyEventKind::Press) => {
                        if !is_locked {
                            let input_text = &mut inputs[current_row];
                            let (number_part, comment_part) = if
                                let Some(pos) = input_text.find('#')
                            {
                                input_text.split_at(pos)
                            } else {
                                (input_text.as_str(), "")
                            };

                            if let Ok(current_value) = number_part.trim().parse::<f64>() {
                                let new_value = current_value + step;
                                let formatted_value = format!("{:.3}", new_value);
                                let spaces = " ".repeat(
                                    number_part.len() - number_part.trim_end().len()
                                );
                                inputs[current_row] = format!(
                                    "{}{}{}",
                                    formatted_value,
                                    spaces,
                                    comment_part
                                );
                            } else if number_part.trim().is_empty() {
                                // 保留三位小数
                                inputs[current_row] = format!("{:.3}{}", step, comment_part);
                            }
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::PageDown, KeyEventKind::Press) => {
                        if !is_locked {
                            let input_text = &mut inputs[current_row];
                            let (number_part, comment_part) = if
                                let Some(pos) = input_text.find('#')
                            {
                                input_text.split_at(pos)
                            } else {
                                (input_text.as_str(), "")
                            };

                            if let Ok(current_value) = number_part.trim().parse::<f64>() {
                                let new_value = (current_value - step).max(0.0);
                                // 保留三位小数
                                let formatted_value = format!("{:.3}", new_value);
                                let spaces = " ".repeat(
                                    number_part.len() - number_part.trim_end().len()
                                );
                                inputs[current_row] = format!(
                                    "{}{}{}",
                                    formatted_value,
                                    spaces,
                                    comment_part
                                );
                            } else if number_part.trim().is_empty() {
                                inputs[current_row] = format!("0.000{}", comment_part);
                            }
                            current_pos = inputs[current_row].len();
                        }
                    }

                    (KeyCode::Left, KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        handle_page_up(
                            current_section.clone(),
                            func_map,
                            inputs,
                            func_toml_path,
                            &mut current_row,
                            &mut current_pos
                        );
                        clear_undo_stack(&undo_stack);
                    }
                    (KeyCode::Right, KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        handle_page_down(
                            current_section.clone(),
                            func_map,
                            inputs,
                            func_toml_path,
                            &mut current_row,
                            &mut current_pos
                        );
                        clear_undo_stack(&undo_stack);
                    }

                    (KeyCode::Char('g'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !inputs[current_row].trim().is_empty() {
                            for i in 0..inputs.len() {
                                let next_index = (current_row + i) % inputs.len();
                                if inputs[next_index].trim().is_empty() {
                                    current_row = next_index;
                                    current_pos = 0;
                                    break;
                                }
                            }
                        }
                    }

                    (KeyCode::Char('f'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            move_cursor_to_next_word(&inputs, &mut current_row, &mut current_pos);
                        }
                    }

                    (KeyCode::Tab, key_event_kind) if
                        (cfg!(target_os = "windows") && key_event_kind == KeyEventKind::Release) ||
                        (cfg!(target_os = "linux") && key_event_kind == KeyEventKind::Press)
                    => {
                        jump_to_input_box(
                            &mut stdout,
                            &inputs,
                            &mut current_row,
                            &mut current_pos
                        )?;
                    }

                    (KeyCode::F(4), KeyEventKind::Press) => {
                        let mut lock_state_guard = lock_state.write().unwrap();
                        *lock_state_guard = !*lock_state_guard;
                        if *lock_state_guard {
                            queue!(stdout, cursor::Hide)?;
                        } else {
                            queue!(stdout, cursor::Show)?;
                            // 调用 align_hash_comments 函数对齐所有输入框中的 `#`
                            align_hash_comments(inputs, &mut current_row, &mut current_pos);
                        }
                    }
                    (KeyCode::F(9), KeyEventKind::Press) => {
                        remove_spaces_before_hash(inputs, &mut current_row, &mut current_pos);
                    }
                    (KeyCode::Char('u'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            push_undo_stack(&undo_stack, &inputs);
                            for input in inputs.iter_mut().take(17) {
                                // 修改此处为17
                                input.clear();
                            }
                            for label in (b'A'..=b'T').map(|c| (c as char).to_string()) {
                                // 修改此处为T
                                variables.remove(&label);
                            }
                            current_pos = 0;
                            current_row = 0;
                        }
                    }

                    (KeyCode::F(12), KeyEventKind::Press) => {
                        if !is_locked {
                            let exe_path = env::current_exe().unwrap();
                            let exe_dir = exe_path.parent().unwrap();
                            let import_txt_path = exe_dir.join(".import.txt");

                            if import_txt_path.exists() {
                                match fs::read_to_string(import_txt_path) {
                                    Ok(content) => {
                                        // 清空当前 section 的前 17 行
                                        for input in inputs.iter_mut().take(17) {
                                            input.clear();
                                        }

                                        // 将 .import.txt 的内容写入当前 section 的前 17 行
                                        let lines: Vec<&str> = content.lines().take(17).collect(); // 只取前17行
                                        for (i, line) in lines.iter().enumerate() {
                                            inputs[i] = line.to_string();
                                        }
                                        current_pos = inputs[current_row].len();
                                    }
                                    Err(e) => {
                                        inputs[current_row].clear();
                                        inputs[current_row].push_str(
                                            &format!("Failed to read .import.txt: {}", e)
                                        );
                                        current_pos = inputs[current_row].len();
                                    }
                                }
                            } else {
                                inputs[current_row].clear();
                                inputs[current_row].push_str(
                                    ".import.txt does not exist in the current directory."
                                );
                                current_pos = inputs[current_row].len();
                            }
                        }
                    }

                    (KeyCode::Char('l'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            push_undo_stack(&undo_stack, &inputs);
                            let label = (b'A' + (current_row as u8)) as char;
                            inputs[current_row].clear();
                            variables.remove(&label.to_string());
                            current_pos = 0;
                        }
                    }
                    (KeyCode::Char('z'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            undo(&undo_stack, inputs, &mut current_row, &mut current_pos);
                        }
                    }
                    (KeyCode::Home, KeyEventKind::Press) => {
                        if !is_locked {
                            *current_section.write().unwrap() = "home".to_string();
                            load_section("home", inputs, func_toml_path);
                            current_pos = 0;
                            current_row = 0;
                            clear_undo_stack(&undo_stack);
                        }
                    }

                    (KeyCode::Char('s'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        save_inputs_to_file(
                            filename,
                            inputs,
                            additional_lines,
                            &current_section.read().unwrap()
                        )?;
                        show_saved_message = true;
                        queue!(buffer, Clear(ClearType::All), cursor::MoveTo(0, 0), Print(title))?;
                        variables.clear();
                        for (i, input) in inputs.iter().enumerate() {
                            let label = (b'A' + (i as u8)) as char;
                            if !input.trim().is_empty() {
                                match evaluate_and_solve(input, &variables, i) {
                                    Ok(res) => {
                                        variables.insert(label.to_string(), res);
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                    }
                    (KeyCode::F(8), KeyEventKind::Press) => {
                        if !is_locked {
                            create_and_load_new_section(
                                &current_section,
                                inputs,
                                func_toml_path,
                                true
                            ).unwrap();
                            current_pos = 0;
                            current_row = 0;
                        }
                    }

                    // Windows 平台独有快捷键, 关于剪切版的操作
                    #[cfg(target_os = "windows")]
                    (KeyCode::Char('c'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if current_row < inputs.len() {
                            let clipboard_value = evaluate_and_solve(
                                &inputs[current_row],
                                &variables,
                                current_row
                            ).unwrap_or_else(|_| "Error".to_string());
                            if clipboard_value != "Error" {
                                let mut clipboard = Clipboard::new().unwrap();
                                clipboard.set_text(clipboard_value).unwrap();

                                // 光标闪烁提示复制完成
                                execute!(stdout, cursor::Hide).unwrap();
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                execute!(stdout, cursor::Show).unwrap();
                            }
                        }
                    }

                    (KeyCode::Char('t'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            current_row = 0;
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::Char('d'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            // 先格式化当前输入框内容
                            inputs[current_row] = format_math_expression(&inputs[current_row]);

                            // 如果当前行不在最后一行，则将当前行内容复制到下一行并跳转到下一行
                            if current_row < inputs.len() - 1 {
                                inputs[current_row + 1] = inputs[current_row].clone();
                                current_row += 1;
                                current_pos = inputs[current_row].len();
                            }
                        }
                    }
                    (KeyCode::Char('a'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked && current_pos > 0 {
                            inputs[current_row] = format_math_expression(&inputs[current_row]);
                            current_pos = 0;
                        }
                    }
                    (KeyCode::Char('b'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            inputs[current_row] = format_math_expression(&inputs[current_row]);
                            current_row = inputs.len() - 1;
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::Char('e'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            inputs[current_row] = format_math_expression(&inputs[current_row]);
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::Down, KeyEventKind::Press) => {
                        if !is_locked && current_row < inputs.len() - 1 {
                            inputs[current_row] = format_math_expression(&inputs[current_row]);
                            current_row += 1;
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::Up, KeyEventKind::Press) => {
                        if !is_locked && current_row > 0 {
                            inputs[current_row] = format_math_expression(&inputs[current_row]);
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
                            push_undo_stack(&undo_stack, &inputs);
                            inputs[current_row].remove(current_pos - 1);
                            current_pos -= 1;
                        }
                    }
                    (KeyCode::Delete, KeyEventKind::Press) => {
                        if !is_locked && current_pos < inputs[current_row].len() {
                            push_undo_stack(&undo_stack, &inputs);
                            inputs[current_row].remove(current_pos);
                        }
                    }
                    (KeyCode::Enter, KeyEventKind::Press) => {
                        if !is_locked {
                            inputs[current_row] = format_math_expression(&inputs[current_row]);
                            let input_command = inputs[current_row].clone().to_lowercase();
                            if input_command.starts_with("rename ") {
                                let new_section_name = input_command
                                    .split_whitespace()
                                    .nth(1)
                                    .unwrap_or("")
                                    .to_string();
                                if !new_section_name.is_empty() {
                                    let current_section_name = current_section
                                        .read()
                                        .unwrap()
                                        .clone();
                                    if
                                        rename_section_in_file(
                                            &current_section_name,
                                            &new_section_name,
                                            func_toml_path
                                        ).is_ok()
                                    {
                                        *current_section.write().unwrap() =
                                            new_section_name.clone();
                                        load_section(&new_section_name, inputs, func_toml_path);
                                        current_pos = 0;
                                        current_row = 0;
                                    } else {
                                        inputs[current_row].clear();
                                        inputs[current_row].push_str("Failed to rename section.");
                                        current_pos = inputs[current_row].len();
                                    }
                                } else {
                                    inputs[current_row].clear();
                                    inputs[current_row].push_str("Invalid new section name.");
                                    current_pos = inputs[current_row].len();
                                }
                            } else if input_command == "new" {
                                create_and_load_new_section(
                                    &current_section,
                                    inputs,
                                    func_toml_path,
                                    false
                                ).unwrap();
                                current_pos = 0;
                                current_row = 0;
                            } else if input_command == "delete" || input_command == "del" {
                                let current_section_name = current_section.read().unwrap().clone();
                                delete_section_from_file(
                                    &current_section_name,
                                    func_toml_path
                                ).unwrap();
                                *current_section.write().unwrap() = "home".to_string();
                                load_section("home", inputs, func_toml_path);
                                current_pos = 0;
                                current_row = 0;
                            } else if input_command == "clone" {
                                create_and_load_new_section(
                                    &current_section,
                                    inputs,
                                    func_toml_path,
                                    true
                                ).unwrap();
                                current_pos = 0;
                                current_row = 0;
                            } else if
                                input_command.starts_with("fc:") &&
                                handle_fc_command(&input_command, inputs, func_map, func_toml_path)
                            {
                                current_pos = inputs[current_row].len();
                                *current_section.write().unwrap() = input_command[3..].to_string();
                            } else if
                                handle_const_command(&input_command, inputs, const_map, current_row)
                            {
                                current_pos = inputs[current_row].len();
                            } else if input_command == "clear" || input_command == "cls" {
                                if !is_locked {
                                    for input in inputs.iter_mut().take(20) {
                                        input.clear();
                                    }
                                    current_pos = 0;
                                    current_row = 0;
                                }
                            } else if cfg!(target_os = "windows") && input_command == "set" {
                                // 获取当前可执行文件所在的目录
                                let exe_path = env::current_exe().unwrap();
                                let exe_dir = exe_path.parent().unwrap();

                                // 设置 notepad3.exe 的路径，位于子目录 notepad3 中
                                let notepad3_path = exe_dir.join("notepad3").join("notepad3.exe");

                                // 设置 .import.txt 的路径，位于当前目录
                                let toml_path = exe_dir.join(".import.txt");

                                // 启动 notepad3.exe 并打开 .import.txt
                                let output = std::process::Command
                                    ::new(notepad3_path)
                                    .arg(toml_path) // 使用当前目录下的 .import.txt
                                    .spawn(); // 非阻塞启动

                                match output {
                                    Ok(_) => {
                                        // inputs[current_row].clear();
                                        // inputs[current_row].push_str("notepad3.exe started successfully!");
                                        inputs[current_row].clear();
                                    }
                                    Err(_) => {
                                        inputs[current_row].clear();
                                        inputs[current_row].push_str(
                                            "Failed to start notepad3/notepad3.exe!"
                                        );
                                    }
                                }
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
                                        inputs[current_row].push_str(
                                            "The rate command was not found!"
                                        );
                                    }
                                }
                                current_pos = inputs[current_row].len();
                            } else if input_command.starts_with("s:") {
                                let command = &input_command[2..].trim();
                                match execute_qalc_command(command) {
                                    Ok(result) => {
                                        inputs[current_row] = result;
                                    }
                                    Err(err) => {
                                        inputs[current_row] = err;
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
                            if (c == 'Z' || c == 'z') && current_row <= 16 {
                                // 修改此处为16
                                inputs[current_row].clear();
                                inputs[current_row].push_str(
                                    "# Global variable Z is limited to the R ~ T area only"
                                );
                                // 修改此处为Q-T
                            } else {
                                push_undo_stack(&undo_stack, &inputs);
                                inputs[current_row].insert(current_pos, c);
                                current_pos += 1;
                            }
                        }
                    }
                    _ => {}
                }
            Event::Mouse(MouseEvent { kind, column, row, .. }) =>
                match kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if !is_locked && (3..inputs.len() + 3).contains(&(row as usize)) {
                            inputs[current_row] = format_math_expression(&inputs[current_row]);
                            let clicked_row = (row as usize) - 3;
                            current_row = clicked_row;
                            current_pos = (column as usize) - (output_width + 9);
                            if current_pos > inputs[current_row].len() {
                                current_pos = inputs[current_row].len();
                            }
                            let label = (b'A' + (current_row as u8)) as char;
                            let result = if inputs[current_row].trim().is_empty() {
                                "".to_string()
                            } else {
                                match
                                    evaluate_and_solve(
                                        &inputs[current_row],
                                        &variables,
                                        current_row
                                    )
                                {
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
                            inputs[current_row] = format_math_expression(&inputs[current_row]);
                            current_row += 1;
                            current_pos = inputs[current_row].len();
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        if !is_locked && current_row > 0 {
                            inputs[current_row] = format_math_expression(&inputs[current_row]);
                            current_row -= 1;
                            current_pos = inputs[current_row].len();
                        }
                    }
                    _ => {}
                }
            _ => {}
        }
    };

    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(result)
}

// 定义 display_current_position 函数
fn display_current_position(
    stdout: &mut io::Stdout,
    current_row: usize,
    current_pos: usize,
    last_row: &mut Option<usize>,
    last_pos: &mut Option<usize>
) -> io::Result<()> {
    // 只有当行号或列号发生变化时才更新显示
    if Some(current_row) != *last_row || Some(current_pos) != *last_pos {
        let cursor_position = (20 + 9 + current_pos) as u16; // 20 是 output_width，9 是额外的偏移
        let mut x_position = 83; // 默认位置

        // 根据 current_row 和 current_pos 动态调整 x_position
        if current_row >= 9 {
            x_position -= 1; // 减少 1
        }
        if current_pos >= 9 {
            x_position -= 1; // 再减少 1
        }

        // 清除原来的行
        execute!(
            stdout,
            cursor::MoveTo(x_position, 26),
            Print(format!("  [{},{}]", current_row + 1, current_pos + 1)),
            cursor::MoveTo(cursor_position, (current_row + 3) as u16) // 将光标移回输入框
        )?;

        // 更新最后显示的位置
        *last_row = Some(current_row);
        *last_pos = Some(current_pos);
    }
    Ok(())
}

/// 从指定文件读取输入数据
fn read_inputs_from_file(filename: &Path) -> Result<(Vec<String>, Vec<String>), io::Error> {
    if !filename.exists() || fs::metadata(filename)?.len() == 0 {
        let initial_content =
            r#"
[home]
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
O = ""
P = ""
Q = ""
R = ""
S = ""
T = ""

[remarks]
R0 = ""
"#;

        fs::write(filename, initial_content)?;
        return Ok((vec!["".to_string(); 20], vec![])); // 修改此处为20
    }

    let mut content = fs::read_to_string(filename)?;
    let mut value: Value = toml
        ::from_str(&content)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    if let Value::Table(ref mut table) = value {
        if !table.contains_key("home") {
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
            initial_0_section.insert("O".to_string(), Value::String("".to_string()));
            initial_0_section.insert("P".to_string(), Value::String("".to_string()));
            initial_0_section.insert("Q".to_string(), Value::String("".to_string()));
            initial_0_section.insert("R".to_string(), Value::String("".to_string()));
            initial_0_section.insert("S".to_string(), Value::String("".to_string()));
            initial_0_section.insert("T".to_string(), Value::String("".to_string()));

            table.insert("home".to_string(), Value::Table(initial_0_section));
            content = toml
                ::to_string(&value)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            fs::write(filename, content.clone())?;
        }
    }

    let value: Value = toml
        ::from_str(&content)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    let mut inputs = vec!["".to_string(); 20]; // 修改此处为20
    let mut additional_lines = vec![];

    if let Value::Table(table) = value {
        if let Some(Value::Table(input_table)) = table.get("home") {
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
                        "O" => 14,
                        "P" => 15,
                        "Q" => 16,
                        "R" => 17,
                        "S" => 18,
                        "T" => 19,
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
        let mut input_table = table
            .get(section)
            .cloned()
            .unwrap_or(Value::Table(toml::map::Map::new()));
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
fn evaluate_and_solve(
    input: &str,
    variables: &HashMap<String, String>,
    current_row: usize
) -> Result<String, String> {
    if current_row <= 10 && input.trim().eq_ignore_ascii_case("z") {
        return Ok("home".to_string());
    }

    if input.starts_with("fc:") {
        return Ok("Import from cfg file".to_string());
    }

    if input.to_lowercase().starts_with("s:") {
        return Ok("Qalculate!".to_string());
    }

    // 调用预处理函数，移除输入中的千位分隔符
    let input_without_commas = remove_thousands_separator(input);

    let input_without_comment = match input_without_commas.find('#') {
        Some(pos) => &input_without_commas[..pos],
        None => &input_without_commas,
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
            let rhs_value = match eval(&replace_percentage(&rhs_replaced.replace(x, "0.0"))) {
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
            let result = (rhs_value - lhs_value) / coefficient;
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
        let mut expression = replace_variables(parts[0].replace(" ", ""), variables);
        expression = expression.replace("/", "*1.0/");

        if expression.contains("z") {
            let global_sum = GLOBAL_SUM.lock().unwrap();
            expression = expression.replace("z", &global_sum.to_string());
        }

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
    re.replace_all(expression, r"$1 * 0.01").to_string()
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
fn calculate_sum_and_count(results: &[String]) -> (f64, usize) {
    let mut sum = 0.0;
    let mut count = 0;
    for result in results.iter().take(17) {
        let cleaned_result = remove_thousands_separator(result);
        match cleaned_result.parse::<f64>() {
            Ok(val) => {
                sum += val;
                count += 1;
            }
            Err(_) => {}
        }
    }

    {
        let mut global_sum = GLOBAL_SUM.lock().unwrap();
        let mut global_count = GLOBAL_COUNT.lock().unwrap();
        *global_sum = sum;
        *global_count = count;
    }

    (sum, count)
}

/// 格式化数值, 在数值中添加千位分隔符以提高可读性
fn format_with_thousands_separator(value: f64) -> String {
    let decimal_places = *DECIMAL_PLACES.lock().unwrap(); // 获取全局控制的小数位数

    // 判断是否为整数，若是整数则不保留小数位
    if value.fract() == 0.0 {
        // 如果是整数，直接返回整数部分
        let formatted_int = format!("{}", value.trunc() as i64); // 转换为整数形式
        return formatted_int
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
    }

    // 如果不是整数，根据 decimal_places 先将小数部分转为字符串
    let formatted_value = format!("{:.1$}", value, decimal_places); // 根据全局变量保留小数位
    let parts: Vec<&str> = formatted_value.split('.').collect();
    let int_part = parts[0];
    let dec_part = parts.get(1).unwrap_or(&"");

    // 移除小数部分末尾的多余零
    let dec_part_trimmed = dec_part.trim_end_matches('0');

    // 如果没有小数位，返回整数部分
    if dec_part_trimmed.is_empty() {
        return int_part
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
    }

    // 如果有小数位，返回整数部分加上处理后的小数部分
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

    format!("{}.{}", formatted_int, dec_part_trimmed)
}

/// 移除格式化数值中的千位分隔符以便进一步处理
fn remove_thousands_separator(value: &str) -> String {
    value.replace(",", "")
}

/// 循环切换 section
fn load_section(section: &str, inputs: &mut Vec<String>, func_toml_path: &Path) {
    if let Ok((func_map, _, _, _, _)) = load_func_commands_from_file(func_toml_path) {
        if let Some(commands) = func_map.get(section) {
            for input in inputs.iter_mut().take(20) {
                // 修改此处为20
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
                    "O" => 14,
                    "P" => 15,
                    "Q" => 16,
                    "R" => 17,
                    "S" => 18,
                    "T" => 19,
                    _ => {
                        continue;
                    }
                };
                inputs[index] = input_value.to_string();
            }
        }
    } else {
        eprintln!("Failed to reload .func.toml");
    }
}

/// 循环切换 section
fn get_next_section(
    func_map: &HashMap<String, HashMap<String, String>>,
    current_section: &str,
    reverse: bool
) -> String {
    let mut keys: Vec<String> = func_map.keys().cloned().collect();
    keys.retain(|k| k != "tui" && k != "remarks" && k != "const");
    keys.sort();
    let current_index = keys
        .iter()
        .position(|k| k == current_section)
        .unwrap_or(0);

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

/// 上翻页
fn handle_page_up(
    current_section: Arc<RwLock<String>>,
    func_map: &mut HashMap<String, HashMap<String, String>>,
    inputs: &mut Vec<String>,
    func_toml_path: &Path,
    current_row: &mut usize,
    current_pos: &mut usize
) {
    // 重新加载 .func.toml 文件
    if
        let Ok((new_func_map, _new_const_map, _custom_color, _custom_attribute, _)) =
            load_func_commands_from_file(func_toml_path)
    {
        *func_map = new_func_map;
    } else {
        eprintln!("Failed to reload .func.toml");
    }

    let new_section = {
        let current_section_name = current_section.read().unwrap().clone();
        get_next_section(func_map, &current_section_name, true)
    };
    *current_section.write().unwrap() = new_section.clone();
    load_section(&new_section, inputs, func_toml_path);

    *current_row = 0;
    *current_pos = 0;
    let mut stdout = io::stdout();
    queue!(stdout, cursor::MoveTo(0, 0)).unwrap();
    stdout.flush().unwrap();
}

/// 下翻页
fn handle_page_down(
    current_section: Arc<RwLock<String>>,
    func_map: &mut HashMap<String, HashMap<String, String>>,
    inputs: &mut Vec<String>,
    func_toml_path: &Path,
    current_row: &mut usize,
    current_pos: &mut usize
) {
    // 重新加载 .func.toml 文件
    if
        let Ok((new_func_map, _new_const_map, _custom_color, _custom_attribute, _)) =
            load_func_commands_from_file(func_toml_path)
    {
        *func_map = new_func_map;
    } else {
        eprintln!("Failed to reload .func.toml");
    }

    let new_section = {
        let current_section_name = current_section.read().unwrap().clone();
        get_next_section(func_map, &current_section_name, false)
    };
    *current_section.write().unwrap() = new_section.clone();
    load_section(&new_section, inputs, func_toml_path);

    *current_row = 0;
    *current_pos = 0;
    let mut stdout = io::stdout();
    queue!(stdout, cursor::MoveTo(0, 0)).unwrap();
    stdout.flush().unwrap();
}

/// 执行 Qalculate! 命令
fn execute_qalc_command(command: &str) -> Result<String, String> {
    let output = if cfg!(target_os = "windows") {
        let exe_path = env::current_exe().unwrap();
        let exe_dir = exe_path.parent().unwrap();
        let command_path = exe_dir.join("qalc/qalc.exe");
        std::process::Command::new(command_path).arg("-t").arg(command).output()
    } else {
        std::process::Command::new("qalc").arg("-t").arg(command).output()
    };

    match output {
        Ok(output) => {
            let result = String::from_utf8_lossy(&output.stdout);
            let trimmed_result = result.trim();
            Ok(trimmed_result.to_string())
        }
        Err(_) => Err("Failed to execute qalc command.".to_string()),
    }
}

/// 将当前状态压入撤销栈
fn push_undo_stack(undo_stack: &Arc<RwLock<Vec<Vec<String>>>>, inputs: &[String]) {
    let mut stack = undo_stack.write().unwrap();
    stack.push(inputs.to_vec());
    if stack.len() > 100 {
        stack.remove(0);
    }
}

fn undo(
    undo_stack: &Arc<RwLock<Vec<Vec<String>>>>,
    inputs: &mut Vec<String>,
    current_row: &mut usize,
    current_pos: &mut usize
) {
    let mut stack = undo_stack.write().unwrap();
    if let Some(last_state) = stack.pop() {
        *inputs = last_state;

        for (i, input) in inputs.iter().enumerate().rev() {
            if !input.is_empty() {
                *current_row = i;
                *current_pos = input.len();
                break;
            }
        }
    }
}

/// 清空撤销栈
fn clear_undo_stack(undo_stack: &Arc<RwLock<Vec<Vec<String>>>>) {
    let mut stack = undo_stack.write().unwrap();
    stack.clear();
}

/// 生成随机 section 名称
fn generate_random_section_name() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_number: u16 = rng.gen_range(10..=99);
    random_number.to_string()
}

/// 更新 .func.toml 文件以添加新的 section
fn add_new_section_to_file(section_name: &str, func_toml_path: &Path) -> Result<(), io::Error> {
    let mut value = if func_toml_path.exists() {
        let content = fs::read_to_string(func_toml_path)?;
        toml::from_str(&content).unwrap_or(Value::Table(toml::map::Map::new()))
    } else {
        Value::Table(toml::map::Map::new())
    };

    if let Value::Table(ref mut table) = value {
        if !table.contains_key(section_name) {
            let mut new_section = toml::map::Map::new();
            for key in [
                "A",
                "B",
                "C",
                "D",
                "E",
                "F",
                "G",
                "H",
                "I",
                "J",
                "K",
                "L",
                "M",
                "N",
                "O",
                "P",
                "Q",
                "R",
                "S",
                "T",
            ].iter() {
                new_section.insert(key.to_string(), Value::String("".to_string()));
            }
            table.insert(section_name.to_string(), Value::Table(new_section));
        }
    }

    let toml_string = toml::to_string(&value).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(func_toml_path, toml_string)?;
    Ok(())
}

/// 删除 section
fn delete_section_from_file(section_name: &str, func_toml_path: &Path) -> Result<(), io::Error> {
    let mut value = if func_toml_path.exists() {
        let content = fs::read_to_string(func_toml_path)?;
        toml::from_str(&content).unwrap_or(Value::Table(toml::map::Map::new()))
    } else {
        Value::Table(toml::map::Map::new())
    };

    if let Value::Table(ref mut table) = value {
        table.remove(section_name);
    }

    let toml_string = toml::to_string(&value).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(func_toml_path, toml_string)?;
    Ok(())
}

/// 克隆 section
fn clone_section_in_file(
    source_section: &str,
    target_section: &str,
    func_toml_path: &Path
) -> Result<(), io::Error> {
    let mut value = if func_toml_path.exists() {
        let content = fs::read_to_string(func_toml_path)?;
        toml::from_str(&content).unwrap_or(Value::Table(toml::map::Map::new()))
    } else {
        Value::Table(toml::map::Map::new())
    };

    if let Value::Table(ref mut table) = value {
        if let Some(source_value) = table.get(source_section).cloned() {
            table.insert(target_section.to_string(), source_value);
        }
    }

    let toml_string = toml::to_string(&value).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(func_toml_path, toml_string)?;
    Ok(())
}

/// 创建并加载新 section
fn create_and_load_new_section(
    current_section: &Arc<RwLock<String>>,
    inputs: &mut Vec<String>,
    func_toml_path: &Path,
    clone: bool
) -> io::Result<()> {
    let current_section_name = current_section.read().unwrap().clone();
    let new_section_name = format!("{}_{}", current_section_name, generate_random_section_name());

    if clone {
        clone_section_in_file(&current_section_name, &new_section_name, func_toml_path)?;
    } else {
        add_new_section_to_file(&new_section_name, func_toml_path)?;
    }

    *current_section.write().unwrap() = new_section_name.clone();
    load_section(&new_section_name, inputs, func_toml_path);
    Ok(())
}

/// 重命名 section
fn rename_section_in_file(
    current_section: &str,
    new_section: &str,
    func_toml_path: &Path
) -> Result<(), io::Error> {
    let mut value = if func_toml_path.exists() {
        let content = fs::read_to_string(func_toml_path)?;
        toml::from_str(&content).unwrap_or(Value::Table(toml::map::Map::new()))
    } else {
        Value::Table(toml::map::Map::new())
    };

    if let Value::Table(ref mut table) = value {
        if let Some(section_value) = table.remove(current_section) {
            table.insert(new_section.to_string(), section_value);
        }
    }

    let toml_string = toml::to_string(&value).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(func_toml_path, toml_string)?;
    Ok(())
}

/// 格式化数学表达式，在运算符前后添加一个空格，并移除多余的空格
fn format_math_expression(expression: &str) -> String {
    let re = Regex::new(r"\s*([+\-*/=])\s*").unwrap();

    // Split the expression at the first occurrence of '#'
    if let Some((before_comment, comment)) = expression.split_once('#') {
        let formatted_before_comment = re.replace_all(before_comment, " $1 ").to_string();
        // Return combined result with the comment part unchanged
        format!("{}#{}", formatted_before_comment, comment)
    } else {
        // If there's no '#', format the entire expression
        re.replace_all(expression, " $1 ").to_string()
    }
}

/// 对齐 # 并更新光标位置
fn align_hash_comments(inputs: &mut Vec<String>, current_row: &mut usize, current_pos: &mut usize) {
    // 找到所有输入框中第一个非行头的 `#` 的最大位置
    let max_hash_pos = inputs
        .iter()
        .filter_map(|input| {
            let hash_pos = input.find('#');
            if let Some(pos) = hash_pos {
                if pos > 0 { Some(pos) } else { None }
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0);

    // 对每个输入框进行处理
    for (i, input) in inputs.iter_mut().enumerate() {
        if let Some(hash_pos) = input.find('#') {
            if hash_pos > 0 {
                // 在 `#` 前添加适当的空格以对齐
                let padding = " ".repeat(max_hash_pos - hash_pos);
                *input = format!("{}{}", &input[..hash_pos], padding) + &input[hash_pos..];

                // 如果是当前行，更新光标位置
                if i == *current_row {
                    *current_pos = hash_pos + padding.len();
                }
            }
        }
    }
}

/// F9 删掉 # 之前的空格一次
fn remove_spaces_before_hash(
    inputs: &mut Vec<String>,
    current_row: &mut usize,
    current_pos: &mut usize
) {
    for (i, input) in inputs.iter_mut().enumerate() {
        if let Some(hash_pos) = input.find('#') {
            // 检查 # 是否位于行头
            if hash_pos == 0 {
                continue; // 如果 # 位于行头，则跳过此行
            }

            // 删除 `#` 前的所有空格
            while hash_pos > 0 && input.as_bytes()[hash_pos - 1] == b' ' {
                input.remove(hash_pos - 1);
            }

            // 如果是当前行，确保光标在 `#` 上
            if i == *current_row {
                *current_pos = input.find('#').unwrap_or(0);
            }
        }
    }
}

/// Tab 单字跳转
fn jump_to_input_box(
    stdout: &mut io::Stdout,
    inputs: &Vec<String>,
    current_row: &mut usize,
    current_pos: &mut usize
) -> io::Result<()> {
    // 移动到第1行第18列
    execute!(stdout, cursor::MoveTo(18, 1), Print("Jump to = "))?;
    stdout.flush()?;

    if
        let Event::Key(
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                ..
            },
        ) = read()?
    {
        if c.is_ascii_alphabetic() && c.to_ascii_uppercase() <= 'T' {
            let target_row = ((c.to_ascii_uppercase() as u8) - b'A') as usize;
            if target_row < inputs.len() {
                *current_row = target_row;
                *current_pos = inputs[*current_row].len();
            }
        }
    }
    //execute!(stdout, cursor::Hide)?;
    execute!(stdout, cursor::Hide, cursor::MoveTo(18, 1), Clear(ClearType::CurrentLine))?;
    Ok(())
}

/// 移动光标到下一个单词后
fn move_cursor_to_next_word(
    inputs: &Vec<String>,
    current_row: &mut usize,
    current_pos: &mut usize
) {
    let current_line = &inputs[*current_row];

    // 如果光标在行尾，则从头开始查找
    if *current_pos >= current_line.len() {
        *current_pos = 0;
    }

    let remaining_text = &current_line[*current_pos..];

    // 使用正则表达式找到下一个单词
    let re = Regex::new(r"\S+").unwrap();
    if let Some(mat) = re.find(remaining_text) {
        *current_pos += mat.end();
    } else {
        *current_pos = current_line.len(); // 如果没有找到下一个单词，则移动到行尾
    }
}
