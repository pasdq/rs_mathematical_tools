use std::collections::HashMap;
use std::fs::{ self, OpenOptions };
use std::io::{ self, BufRead, BufReader, Write };
use std::path::Path;
use std::sync::{ Arc, Mutex };
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
    style::{ Attribute, SetAttribute, Color, Print, SetForegroundColor, ResetColor },
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

/// 处理命令以从预定义的函数集合中加载函数定义如果命令匹配一个函数键, 则用相应的函数值更新输入列表
fn handle_func_command(
    command: &str,
    inputs: &mut Vec<String>,
    func_map: &HashMap<String, HashMap<String, String>>
) -> bool {
    if command.starts_with("fc.") {
        // fc. 是三个字符
        let key = &command[3..];
        if let Some(commands) = func_map.get(key) {
            // 先清除前13个输入区域, 只保留最后一个 N
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
    }
    false
}

/// 从 TOML 文件中加载函数命令, 并将它们解析为嵌套的 HashMap每个函数键映射到另一个包含命令定义的 HashMap
fn load_func_commands_from_file(
    filename: &Path
) -> Result<HashMap<String, HashMap<String, String>>, io::Error> {
    if !filename.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(filename)?;
    let value: Value = toml::from_str(&content)?;
    let mut func_map = HashMap::new();

    if let Value::Table(table) = value {
        for (key, value) in table {
            if let Value::Table(command_table) = value {
                let mut commands = HashMap::new();
                for (command_key, command_value) in command_table {
                    if let Value::String(command_string) = command_value {
                        commands.insert(command_key, command_string);
                    }
                }
                func_map.insert(key.to_lowercase(), commands);
            }
        }
    }

    Ok(func_map)
}

/// 应用程序的主要入口点它从文件中读取函数命令, 解析命令行参数, 初始化输入状态, 并在启用终端设置的情况下运行主要应用循环
fn main() -> io::Result<()> {
    // 读取 .func.toml 文件中的命令
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().unwrap();
    let func_toml_path = exe_dir.join(".func.toml");
    let func_map = load_func_commands_from_file(&func_toml_path)?;

    // 解析命令行参数
    let args = Args::parse();
    let filename = args.filename.map(PathBuf::from).unwrap_or_else(|| exe_dir.join(".last.txt"));

    // 读取输入
    let (mut inputs, additional_lines) = read_inputs_from_file(&filename).unwrap_or_else(|_| (
        vec!["".to_string(); 14],
        vec![],
    ));

    let lock_state = Arc::new(Mutex::new(false));

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;

    let result = run_app(
        &filename,
        &mut inputs,
        &additional_lines,
        Arc::clone(&lock_state),
        &func_map
    );

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

    if let Err(e) = save_inputs_to_file(&filename, &inputs, &additional_lines) {
        eprintln!("Error saving inputs to file: {}", e);
    }

    result
}

/// 应用程序的核心循环它处理用户输入、更新显示、评估表达式和处理命令同时管理终端显示和交互
fn run_app(
    filename: &Path,
    inputs: &mut Vec<String>,
    additional_lines: &Vec<String>,
    lock_state: Arc<Mutex<bool>>,
    func_map: &HashMap<String, HashMap<String, String>>
) -> io::Result<()> {
    let mut stdout = io::stdout();
    let mut variables = HashMap::new();
    let mut current_row = 0;
    let mut current_pos = 0;
    let input_width = 47;
    let output_width = 23;
    let title = " RS Mathematical Tools                                                   V1.2.1 ";
    let heade = "                     Result  =  Mathematical Expression";
    let foote = " Keyword: About | Rate | KK | fc.1-9                   https://github.com/pasdq ";
    let saved = "[ ---------------------------- Recalculate & saved! -------------------------- ]";
    let mut show_saved_message = false;

    queue!(
        stdout,
        Clear(ClearType::All),
        SetAttribute(Attribute::Reverse),
        cursor::MoveTo(0, 0),
        Print(title),
        ResetColor
    )?;
    queue!(
        stdout,
        SetAttribute(Attribute::Bold),
        SetForegroundColor(Color::Blue),
        cursor::MoveTo(0, 2),
        Print(heade),
        ResetColor
    )?; // 新插入的行
    stdout.flush()?;

    loop {
        let is_locked = *lock_state.lock().unwrap();
        let (term_width, _) = size()?;
        let mut results = Vec::new();
        let mut buffer = Vec::new();

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
                            "Import from cfg file".to_string() // 不显示 "Error"
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
                        cursor::MoveTo(0, (i + 3) as u16), // 行号 +1
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
                            SetForegroundColor(Color::Green),
                            SetAttribute(Attribute::Underlined),
                            cursor::MoveTo(0, (i + 3) as u16), // 行号 +1
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
                            SetForegroundColor(if i >= 12 { Color::Blue } else { Color::Green }),
                            cursor::MoveTo(0, (i + 3) as u16), // 行号 +1
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
                        cursor::MoveTo(0, (i + 3) as u16), // 行号 +1
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
                        cursor::MoveTo(0, (i + 3) as u16), // 行号 +1
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
            cursor::MoveTo(0, (inputs.len() + 4) as u16), // 行号 +1
            Print(" ".repeat(term_width as usize)),
            cursor::MoveTo(0, (inputs.len() + 5) as u16), // 行号 +1
            Print(" ".repeat(term_width as usize)),
            SetForegroundColor(Color::Blue),
            cursor::MoveTo(17, (inputs.len() + 4) as u16), // 行号 +1
            Print(format!("(A - L) Sum = {}", format_with_thousands_separator(sum))),
            cursor::MoveTo(21, (inputs.len() + 5) as u16), // 行号 +1
            Print(format!("Average = {}", format_with_thousands_separator(average))),
            cursor::MoveTo(0, (inputs.len() + 8) as u16), // 行号 +1
            ResetColor,
            SetAttribute(Attribute::Reverse),
            Print(foote),
            ResetColor,
            cursor::MoveTo(22, 20), // 行号 +1
            SetForegroundColor(if is_locked { Color::Red } else { Color::Green }),
            Print(
                format!("Status = {} (F4 Status Switch)", if is_locked {
                    "Locked"
                } else {
                    "Opened"
                })
            ),
            ResetColor
        )?;

        if show_saved_message {
            queue!(
                buffer,
                cursor::MoveTo(0, 17), // 行号 +1
                SetForegroundColor(Color::DarkYellow),
                Print(saved),
                ResetColor
            )?;
            show_saved_message = false;
        } else {
            queue!(buffer, cursor::MoveTo(0, 17), Print(" ".repeat(term_width as usize)))?; // 行号 +1
        }

        for (i, line) in additional_lines.iter().enumerate() {
            queue!(buffer, cursor::MoveTo(0, (inputs.len() + 9 + i) as u16), Print(line))?; // 行号 +1
        }

        if is_locked {
            queue!(buffer, cursor::Hide)?;
        } else {
            let cursor_position = (output_width + 9 + current_pos) as u16;
            queue!(
                buffer,
                cursor::MoveTo(cursor_position, (current_row + 3) as u16), // 行号 +1
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
                    (KeyCode::F(4), KeyEventKind::Press) => {
                        let mut lock_state_guard = lock_state.lock().unwrap();
                        *lock_state_guard = !*lock_state_guard;
                        if *lock_state_guard {
                            queue!(stdout, cursor::Hide)?;
                        } else {
                            queue!(stdout, cursor::Show)?;
                        }
                    }
                    (KeyCode::Char('u'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            // 仅清空前面13个输入区域, 保留最后一个 N
                            for input in inputs.iter_mut().take(13) {
                                input.clear();
                            }
                            // 清除对应的变量
                            for label in (b'A'..=b'L').map(|c| (c as char).to_string()) {
                                variables.remove(&label);
                            }
                            current_pos = 0;
                            current_row = 0;
                        }
                    }
                    (KeyCode::Char('l'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            let label = (b'A' + (current_row as u8)) as char;
                            inputs[current_row].clear();
                            variables.remove(&label.to_string()); // 清除对应的变量
                            current_pos = 0;
                        }
                    }
                    (KeyCode::F(5), key_event_kind) if
                        (cfg!(target_os = "windows") && key_event_kind == KeyEventKind::Release) ||
                        (cfg!(target_os = "linux") && key_event_kind == KeyEventKind::Press)
                    => {
                        // 保存当前输入内容到 .last.txt
                        save_inputs_to_file(filename, inputs, additional_lines)?;

                        show_saved_message = true; // 设置显示“saved”消息

                        queue!(buffer, Clear(ClearType::All), cursor::MoveTo(0, 0), Print(title))?;
                        queue!(
                            stdout,
                            SetAttribute(Attribute::Bold),
                            SetForegroundColor(Color::Blue),
                            cursor::MoveTo(0, 2),
                            Print(heade),
                            ResetColor
                        )?; // 新插入的行
                        variables.clear(); // 刷新前清除变量
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
                    (KeyCode::Char('t'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            current_row = 0;
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::Char('a'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            current_pos = 0;
                        }
                    }
                    (KeyCode::Char('b'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            current_row = inputs.len() - 1;
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::Char('e'), KeyEventKind::Press) if
                        modifiers.contains(KeyModifiers::CONTROL)
                    => {
                        if !is_locked {
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::Down | KeyCode::Tab, KeyEventKind::Press) => {
                        if !is_locked {
                            current_row = (current_row + 1) % inputs.len();
                            current_pos = inputs[current_row].len();
                        }
                    }
                    (KeyCode::Up, KeyEventKind::Press) => {
                        if !is_locked {
                            if current_row == 0 {
                                current_row = inputs.len() - 1;
                            } else {
                                current_row -= 1;
                            }
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
                            if handle_func_command(&input_command, inputs, func_map) {
                                current_pos = inputs[current_row].len();
                            } else if input_command == "about" {
                                inputs[current_row].clear();
                                inputs[current_row].push_str("# RS Mathematical Tools V1.1.0");
                                current_pos = inputs[current_row].len();
                            } else if input_command == "kk" {
                                inputs[current_row].clear();
                                inputs[current_row].push_str("1000.0 # Thousand");
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
                                        let trimmed_result = result.trim(); // 移除前后的空白和换行符
                                        inputs[current_row].push_str(trimmed_result);
                                    }
                                    Err(_) => {
                                        inputs[current_row].push_str(
                                            "The rate command was not found!"
                                        );
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
                        let clicked_row = (row as usize) - 3; // 行号偏移增加1
                        if !is_locked && (3..inputs.len() + 3).contains(&(row as usize)) {
                            current_row = clicked_row;
                            current_pos = (column as usize) - (output_width + 9);
                            if current_pos > inputs[current_row].len() {
                                current_pos = inputs[current_row].len();
                            }
                            // 仅在点击时重绘当前行
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
                                cursor::MoveTo(0, (current_row + 3) as u16), // 行号偏移增加1
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
                        if !is_locked {
                            current_row = (current_row + 1) % inputs.len();
                            current_pos = inputs[current_row].len();
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        if !is_locked {
                            if current_row == 0 {
                                current_row = inputs.len() - 1;
                            } else {
                                current_row -= 1;
                            }
                            current_pos = inputs[current_row].len();
                        }
                    }
                    _ => {}
                }
            _ => {}
        }
    }

    Ok(())
}

/// 从指定文件读取输入数据如果文件不存在或为空, 则初始化输入列表和附加行
fn read_inputs_from_file(filename: &Path) -> Result<(Vec<String>, Vec<String>), io::Error> {
    if !filename.exists() || fs::metadata(filename)?.len() == 0 {
        fs::File::create(filename)?;
        return Ok((vec!["".to_string(); 14], vec![]));
    }

    let file = fs::File::open(filename)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut inputs = vec!["".to_string(); 14];
    for i in 0..14 {
        if let Some(Ok(line)) = lines.next() {
            if let Some(pos) = line.find('=') {
                inputs[i] = line[pos + 1..].to_string();
            } else {
                inputs[i] = line;
            }
        }
    }

    let additional_lines: Vec<String> = lines.filter_map(Result::ok).collect();
    Ok((inputs, additional_lines))
}

/// 将当前输入数据和附加行的状态保存到指定文件
fn save_inputs_to_file(
    filename: &Path,
    inputs: &[String],
    additional_lines: &[String]
) -> Result<(), io::Error> {
    let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(filename)?;

    for (i, input) in inputs.iter().enumerate() {
        let label = (b'A' + (i as u8)) as char;
        writeln!(file, "{}={}", label, input)?;
    }

    for line in additional_lines {
        writeln!(file, "{}", line)?;
    }

    Ok(())
}

/// 评估和求解输入中提供的数学表达式或方程支持线性方程和带变量替换的一般表达式
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

/// 计算结果列表中有效数值结果的总和和数量最多处理前 12 个结果
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
