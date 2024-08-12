
use std::error::Error;
use std::env;
use pcre2::bytes::Regex;
use encoding_rs::GBK;

pub fn run(config: Config) -> Result<Vec<(String, String)>, Box<dyn Error>> {
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

pub fn search<'a>(query: &str, contents: &'a str) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let regex = Regex::new(query)?;
    let mut matches = vec![];

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
                        
                    // let sum: u32 = idcard.clone()[..17].parse().unwrap();
                    // let weights:Vec<&u32> = [7,9,10,5,8,4,2,1,6,3,7,9,10,5,8,4,2];
                    

                    // let checksum = 
                }

                let start = m.start();
                let end = m.end();
                // 根据捕获的起始和结束位置获取匹配的字符串
                let match_str = String::from_utf8_lossy(&line.as_bytes()[start..end]).to_string();
                matches.push(((index + 1).to_string(), match_str));
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