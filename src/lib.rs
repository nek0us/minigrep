
use std::error::Error;
use std::env;
use pcre2::bytes::Regex;
use encoding_rs::GBK;

use std::sync::{Arc,Mutex};
use tokio::sync::{Semaphore};
use tokio;

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub matched_text: String,
    pub file_name: String,
    pub line_number: String,
    pub origin_text: String,
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