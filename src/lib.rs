use std::env;
use std::fs::File;
use std::io::Write;
use std::error::Error;
use pcre2::bytes::Regex;
use regex::Regex as Regex_origin;

use tokio;
use tokio::sync::Semaphore;
use std::cell::RefCell;
use std::sync::{Arc,Mutex};

use encoding_rs::GBK;
use html_escape::encode_text;

use native_windows_gui::EventData;
extern crate native_windows_gui as nwg;  

mod text;
use text::{HTML_FOOTER, HTML_HEAD};

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub matched_text: String,
    pub file_name: String,
    pub line_number: String,
    pub origin_text: String,
}


pub fn export_to_html(
    matched_text_storage: Arc<Vec<String>>, 
    regex_list: Arc<Vec<String>>,
    file_name_storage: Arc<Vec<String>>,
    copy_storage: Arc<Vec<String>>,
) -> String {
    let mut html_content = String::new();
    let mut sidebar_content = String::new();
    let mut regex_content = String::new();
    regex_content.push_str("<div id=\"regex-list\" class=\"regex-section\"><h2>使用的正则表达式</h2><ul>");
    for regex in regex_list.iter() {
        regex_content.push_str(&format!("<li>{}</li>", encode_text(regex)));
    }
    regex_content.push_str("</ul></div>");

    for (index, file_name) in file_name_storage.iter().enumerate() {
        if let Some(full_text) = copy_storage.get(index) {
            if let Some(matched_text) = matched_text_storage.get(index) {
                let match_id = format!("match-{}", index);

                // 添加到侧边栏的导航项
                sidebar_content.push_str(&format!(
                    "<p><a href=\"javascript:void(0);\" onclick=\"scrollToMatch('{}')\">{} {} </a></p>", // ({}) </a></p>
                    match_id, &index.clone() + 1, matched_text //, file_name
                ));

                // 正确展示文件路径和匹配值
                html_content.push_str(&format!(
                    "<div id=\"{}\" class=\"file-section\"><h2>{} {}</h2><p>文件路径: {}</p>",
                    match_id, &index.clone() + 1, matched_text, file_name
                ));

                let mut match_positions = Vec::new();
                let mut search_start = 0;

                while let Some(byte_pos) = full_text[search_start..].find(matched_text) {
                    let byte_pos = byte_pos + search_start;
                    let start = full_text[..byte_pos].chars().count();
                    let end = start + matched_text.chars().count();
                    match_positions.push((start, end));
                    search_start = byte_pos + matched_text.len();
                }

                // 只显示第一段匹配行上下文
                if let Some((start, end)) = match_positions.first() {
                    let left_context_start = start.saturating_sub(50);
                    let right_context_end = std::cmp::min(end + 50, full_text.chars().count());

                    let left_context: String = full_text.chars().skip(left_context_start).take(start - left_context_start).collect();
                    let matched_context: String = full_text.chars().skip(start.clone()).take(end - start).collect();
                    let right_context: String = full_text.chars().skip(end.clone()).take(right_context_end - end).collect();

                    html_content.push_str(&format!(
                        r#"
                            <div class="match">
                                <div class="code-container" data-id="context-{}" onclick="toggleFullLine('context-{}')">
                                    <div class="code-left">{}</div>
                                    <div class="highlight">{}</div>
                                    <div class="code-right">{}</div>
                                </div>
                            </div>
                        "#,
                        index,
                        index,
                        left_context,
                        matched_context,
                        right_context
                    ));
                }

                // 展开全文，显示所有匹配值高亮

                let highlighted_full_context = {
                    let mut highlighted = String::new();
                    let mut current_index = 0; // 当前字符索引
                
                    // 遍历全文字符
                    for (_i, ch) in full_text.chars().enumerate() {
                        if let Some(&(start, _end)) = match_positions.iter().find(|&&(s, _e)| s == current_index) {
                            // 如果当前位置匹配开始，则插入高亮标签
                            if current_index == start {
                                highlighted.push_str("<span class=\"highlight\">");
                            }
                        }
                        
                        // 添加当前字符到结果
                        highlighted.push(ch);
                
                        // 如果当前位置是匹配结束，则关闭高亮标签
                        if match_positions.iter().any(|&(_s, e)| e == current_index + 1) {
                            highlighted.push_str("</span>");
                        }
                
                        current_index += 1; // 更新字符索引
                    }
                
                    highlighted
                };
                
                html_content.push_str(&format!(
                    r#"
                        <div id="context-{}" class="full-line">
                            <div class="line-content">{}</div>
                        </div>
                    "#,
                    index,
                    highlighted_full_context
                ));

                html_content.push_str(&format!("<p>总共匹配 {} 个值</p>", match_positions.len()));
            }

            html_content.push_str("</div>");
        }
    }

    format!("{}{}</br></div><div id=\"content\">{}{}</div>{}", HTML_HEAD, sidebar_content, regex_content, html_content, HTML_FOOTER)


}

pub fn point_to_details(evt_data: EventData,re: Regex_origin,copy_storage: Arc<Vec<String>>,matched_text_storage: Arc<Vec<String>>, file_name_storage: Arc<Vec<String>>,origin_text: Arc<RefCell<nwg::RichTextBox>>,origin_file: Arc<RefCell<nwg::TextInput>>) {
    let (index,_) = evt_data.on_list_view_item_index();
    // 验证索引是否有效，防止崩溃
    if index < copy_storage.len() {
        if let Some(full_text) = copy_storage.get(index) {
            // 使用正则表达式替换 Unicode 转义字符
            let unescaped_text = re.replace_all(full_text, |caps: &regex::Captures| {
                let code_point = u16::from_str_radix(&caps[1], 16).unwrap();
                char::from_u32(u32::from(code_point)).unwrap().to_string()
            });

            // 将 \r\n 替换为 \n，标准化换行符，解决三行高亮异常的问题
            let unescaped_text = unescaped_text.replace("\r\n", "\n");

            origin_text.borrow_mut().set_text(&unescaped_text);

            // 新增代码：定位并选中匹配的文本
            if let Some(matched_text) = matched_text_storage.get(index) {
                let mut match_positions = Vec::new(); // 存储所有匹配的字符位置
            
                let mut search_start = 0;
                while let Some(byte_pos) = unescaped_text[search_start..].find(matched_text) {
                    let byte_pos = byte_pos + search_start;
                    // 将字节位置转换为字符索引
                    let start = unescaped_text[..byte_pos].chars().count();
                    let end = start + matched_text.chars().count();
                    match_positions.push((start, end));
            
                    // 更新搜索起点，防止死循环
                    search_start = byte_pos + matched_text.len();
                }
            
                // 设置文本内容
                origin_text.borrow_mut().set_text(&unescaped_text);
            
                // 遍历每个匹配值，应用高亮格式
                for &(start, end) in &match_positions {
                    // 设置选中范围
                    origin_text.borrow_mut().set_selection(start as u32..end as u32);
            
                    // 设置字符格式（高亮显示）
                    let c1 = nwg::CharFormat {
                        text_color: Some([255, 0, 0]), // 红色字体
                        effects: None,
                        y_offset: None,
                        height: None,
                        font_face_name: None,
                        underline_type: None,
                    };
                    origin_text.borrow_mut().set_char_format(&c1);
                }
                origin_text.borrow_mut().set_selection(0..0);
                origin_text.borrow_mut().set_focus();
                // 将光标移动到第一个匹配值的位置，确保可见
                if let Some(&(_, first_end)) = match_positions.first() {
                    // 获取文本的总长度
                    let total_length = unescaped_text.chars().count();
            
                    // 计算行剩余长度
                    let line_remaining_length = total_length - first_end;
            
                    // 定义光标新位置
                    let new_caret_pos = if line_remaining_length > 20 {
                        first_end + 19
                        } else {
                            total_length - 1
                        };
            
                    // 将选中范围设置为零长度，在新光标位置，以移动光标
                    origin_text.borrow_mut().set_selection(new_caret_pos as u32..new_caret_pos as u32);
                    origin_text.borrow_mut().set_focus();
                }
            
                // 更新 file_name，添加匹配值数量提示
                let match_count = match_positions.len();
                let file_name_with_count = if match_count > 1 {
                    format!("({}个匹配值) | {}", match_count, file_name_storage.get(index).unwrap_or(&"".to_string()))
                } else {
                    file_name_storage.get(index).unwrap_or(&"".to_string()).to_string()
                };
                // if let Some(file_name) = file_name_storage.get(index) {
                //     origin_file.borrow_mut().set_text((file_name_with_count + file_name).as_str());
                // }
                origin_file.borrow_mut().set_text(&file_name_with_count);
            }
            
        }
    }
}

pub fn save_html(file_dialog_save: Arc<RefCell<nwg::FileDialog>>,dyn_tis: Arc<RefCell<nwg::Label>>, handle: &nwg::ControlHandle,html_content: String) {
    // 将生成的HTML保存到文件
    if file_dialog_save.borrow_mut().run(Some(handle)) {
        if let Ok(path) =file_dialog_save.borrow_mut().get_selected_item() {
            let mut path = path.into_string().unwrap_or_else(|_| String::new());

        if !path.ends_with(".html") {
            path.push_str(".html");
        }
            let mut file: File = match File::create(&path) {
                Ok(f) => f,
                Err(e) => {
                    println!("无法创建文件: {}", e);
                    return;
                }
            };
        
            if let Err(e) = file.write(html_content.as_bytes()) {
                println!("写入文件失败: {}", e);
            } else {
                dyn_tis.borrow_mut().set_text(format!("结果已保存到 {}", &path).as_str());
            }
        }
    }

}

pub fn search_in_file_contents_sync(res : Arc<Mutex<Vec<MatchResult>>>, regex_list: Arc<Vec<String>>, contents: &str, file_name: &str) {
    let re_list = Arc::clone(&regex_list);
    for query in re_list.iter() {
        let mut query_regex = query.clone();
        // let handle = handles.clone();
        // 这里替换了发布包扫描默认规则库第一条，对于class代码扫描时强制启用引号检测
        if file_name.ends_with(".class") | file_name.ends_with(".java"){
            if query == r#"((P|p)((A|a)(S|s)(S|s))?(W|w)((O|o)(R|r))?(D|d)|(K|k)(E|e)(Y|y)|(E|e)(N|n)(C|c)(R|r)(Y|y)(P|p)(T|t)|(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)|(A|a)(U|u)(T|t)(H|h)((O|o)(R|r)(I|i)(Z|z)(A|a)(T|t)(I|i)(O|o)(N|n))?)\s?[\"\']?(=|:)+\s?[\"\']?[a-zA-Z0-9\@\.]+[\"\']?"# {
                query_regex = String::from(r#"((P|p)((A|a)(S|s)(S|s))?(W|w)((O|o)(R|r))?(D|d)|(K|k)(E|e)(Y|y)|(E|e)(N|n)(C|c)(R|r)(Y|y)(P|p)(T|t)|(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)|(A|a)(U|u)(T|t)(H|h)((O|o)(R|r)(I|i)(Z|z)(A|a)(T|t)(I|i)(O|o)(N|n))?)\s?[\"\']?(=|:)+\s?[\"\']+[a-zA-Z0-9\@\.]+[\"\']+"#);
            }
            
        }
        let config = Config {
            query: query_regex,
            contents: contents.to_string(),
            ignore_case: false,
        };
        let file_name_clone = String::from(file_name);
        let res_clone = res.clone();
        // let a: &Vec<tokio::task::JoinHandle<()>> = &*handles.borrow_mut().borrow();
        {
            
            if let Ok(matches) = run(config) {
                for (line_number, matched_text, origin_text) in matches{

                    {
                        let mut m = res_clone.lock().unwrap();
                        m.push(MatchResult {
                            file_name: file_name_clone.clone(),
                            line_number,
                            matched_text,
                            origin_text
                            }   
                        );
                    }
                }
            }
            
        }
    }
}


pub fn search_in_file_contents(res : Arc<Mutex<Vec<MatchResult>>>,handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,  semaphore : Arc<Semaphore>,  regex_list: Arc<Vec<String>>, contents: &str, file_name: &str) {

    let re_list = Arc::clone(&regex_list);
    
    for query in re_list.iter() {
        let mut query_regex = query.clone();
        // 这里替换了发布包扫描默认规则库第一条，对于class代码扫描时强制启用引号检测
        if file_name.ends_with(".class") | file_name.ends_with(".java"){
            if query == r#"((P|p)((A|a)(S|s)(S|s))?(W|w)((O|o)(R|r))?(D|d)|(K|k)(E|e)(Y|y)|(E|e)(N|n)(C|c)(R|r)(Y|y)(P|p)(T|t)|(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)|(A|a)(U|u)(T|t)(H|h)((O|o)(R|r)(I|i)(Z|z)(A|a)(T|t)(I|i)(O|o)(N|n))?)\s?[\"\']?(=|:)+\s?[\"\']?[a-zA-Z0-9\@\.]+[\"\']?"# {
                query_regex = String::from(r#"((P|p)((A|a)(S|s)(S|s))?(W|w)((O|o)(R|r))?(D|d)|(K|k)(E|e)(Y|y)|(E|e)(N|n)(C|c)(R|r)(Y|y)(P|p)(T|t)|(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)|(A|a)(U|u)(T|t)(H|h)((O|o)(R|r)(I|i)(Z|z)(A|a)(T|t)(I|i)(O|o)(N|n))?)\s?[\"\']?(=|:)+\s?[\"\']+[a-zA-Z0-9\@\.]+[\"\']+"#);
            }
            
        }
        let config = Config {
            query: query_regex,
            contents: contents.to_string(),
            ignore_case: false,
        };
        let file_name_clone = String::from(file_name);
        let res_clone = res.clone();
        {
            let permit = Arc::clone(&semaphore);
            //handles_ref.push(
            let handle = tokio::spawn(async move {
            if let Ok(matches) = run(config) {
                for (line_number, matched_text, origin_text) in matches{

                    {
                        let mut m = res_clone.lock().unwrap();
                        m.push(MatchResult {
                            file_name: file_name_clone.clone(),
                            line_number,
                            matched_text,
                            origin_text
                            }   
                        );
                    }
                }
            }
            drop(permit); // 释放许可
            });//);
            {
                let mut hs = handles.lock().unwrap();
                hs.push(handle);
            }
        }
    }

}

fn run(config: Config) -> Result<Vec<(String, String, String)>, Box<dyn Error>> {
    match search(&config.query, &config.contents) {
        Ok(result) => Ok(result),
        Err(_) => {
            // 尝试使用GBK编码重新匹配
            let gbk_encoded = GBK.encode(&config.contents).0;
            let gbk_contents = String::from_utf8_lossy(gbk_encoded.as_ref()).to_string();
            search(&config.query, &gbk_contents)
        }
    }
}

fn search<'a>(query: &str, contents: &'a str) -> Result<Vec<(String, String, String)>, Box<dyn Error>> {
    let regex = Regex::new(query)?;
    let mut matches = vec![];
    let lines: Vec<&str> = contents.lines().collect();
    for (index, line) in contents.lines().enumerate() {
        if let Some(caps) = regex.captures(line.as_bytes())? {
            if let Some(m) = caps.get(0) { // 获取第一个捕获组
                if query == r"[a-zA-Z0-9\*]+\@[a-zA-Z0-9]+\.[a-zA-Z]+" {
                    if let Some(fa) = Regex::new(r"[a-zA-Z0-9]+\@[a-zA-Z0-9]+\.[a-zA-Z]+")?.captures(line.as_bytes())? {
                        if let Some(f) = fa.get(0) {
                            if f != m {
                                continue;
                            }
                        }
                        
                    } else {
                        continue;
                    }
                } else if query == r"(?<!\d)(\d{17}[Xx]|\d{18})(?!\d)" {
                    // 身份证
                    let start = m.start();
                    let end = m.end();
                    let idcard = String::from_utf8_lossy(&line.as_bytes()[start..end]).to_string();
                    let y:u32 = idcard.clone()[6..10].parse().unwrap();
                    if y < 1900 || y > 2025 {
                        continue;
                    }
                    let m:u32 = idcard.clone()[10..12].parse().unwrap();
                    if m < 1 || m > 12 {
                        continue;
                    }
                    let d:u32 = idcard.clone()[12..14].parse().unwrap();
                    if d < 1 || d > 31 {
                        continue;
                    }
                    // 将身份证号码的前17位转换为数字
                    let mut digits = Vec::new();
                    for c in idcard.chars().take(17) {
                        if let Some(digit) = c.to_digit(10) {
                            digits.push(digit);
                        } else {
                            panic!("Invalid character in ID card");
                        }
                    }

                    // 计算权重乘积之和
                    // 计算权重乘积之和
                    let mut weight_sum: u32 = 0;
                    let weights = [7,9,10,5,8,4,2,1,6,3,7,9,10,5,8,4,2];

                    for (i,d) in digits.iter().enumerate() {
                        weight_sum += d * weights[i];
                    };
                    let checksum: u32 = (12 - (weight_sum % 11)) % 11;
                    
                    let mut result: bool = false;
                    let last_digit: char = idcard.as_bytes()[17] as char;
                    if checksum == 10 {
                        if last_digit == 'X' || last_digit == 'x' {
                            result = true;
                        }
                    } else {
                        if last_digit.to_string() == checksum.to_string() {
                            result = true;
                        }
                    }
                    if !result {
                        continue;
                    }
                        
                }

                let start = m.start();
                let end = m.end();
                // 根据捕获的起始和结束位置获取匹配的字符串
                let match_str = String::from_utf8_lossy(&line.as_bytes()[start..end]).to_string();

                // 获取上一行和下一行
                let prev_line = if index > 0 { lines[index - 1] } else { "" };
                let next_line = if index < lines.len() - 1 { lines[index + 1] } else { "" };
                let combined_text = format!(
                    "{}\r\n{}\r\n{}",
                    prev_line,
                    line,
                    next_line
                );
                matches.push(((index + 1).to_string(), match_str, combined_text));
            }
        }
    }

    Ok(matches)
}


pub struct Config {
    pub query: String,
    // pub file_path: String,
    pub contents: String,  // 添加一个字段用于存储文件内容
    pub ignore_case: bool,
}

impl Config {
    pub fn build(
        mut args: impl Iterator<Item = String>,
    ) -> Result<Config, &'static str> {
        // if args.size_hint() < 3{
        //     return Err("没有输入要查找的内容");
        // }
        args.next();
        let query = match args.next() {
            Some(arg) => arg,
            None => return Err("没有输入要查找的内容")
        }; 
        let contents = match args.next() {
            Some(arg) => arg,
            None => return Err("没有输入要查找的文件")
        };

        let ignore_case = env::var("IGNORE_CASE").is_ok();

        Ok(Config { query,contents,ignore_case })
    }
}

// pub fn search<'a>(query: &str, contents: &'a str) -> Vec<&'a (usize,&str)> {
//     // let mut results = Vec::new();
//     // for line in contents.lines() {
//     //     if line.contains(query) {
//     //         results.push(line);
//     //     }
//     // }
//     // results
//     println!("使用正则匹配");
//     contents
//     .lines()
//     .enumerate()
//     .filter(|(index,line)|Regex::new(query).unwrap().is_match(line.as_bytes()).unwrap())
//     .collect()
// }

pub fn search_case_insensitive<'a>(
    query: &str,
    contents: &'a str,
) -> Vec<&'a str> {
    let query = query.to_lowercase();
    let mut results = Vec::new();

    for line in contents.lines() {
        if line.to_lowercase().contains(&query) {
            results.push(line);
        }
    }
    results
}

// #[cfg(test)]
// mod tests {
//     use super::*;

// //     #[test]
// //     fn one_result() {
// //         let query = "duct";
// //         let contents = "\
// // Rust:
// // safe, fast, productive.
// // Pick three.";

// //         assert_eq!(vec!["safe, fast, productive."], search(query, contents));
// //     }

//     #[test]
//     fn case_sensitive() {
//         let query = "duct";
//         let contents = "\
// Rust:
// safe, fast, productive.
// Pick three.
// Duct tape.";

//         assert_eq!(vec!["safe, fast, productive."], search(query, contents));

    


//     }

//     #[test]
//     fn case_insensitive() {
//         let query = "rUsT";
//         let contents = "\
// Rust:
// safe, fast, productive.
// Pick three.
// Trust me.";

//         assert_eq!(
//             vec!["Rust:", "Trust me."],
//             search_case_insensitive(query, contents)
//         );
//     }
    
// }