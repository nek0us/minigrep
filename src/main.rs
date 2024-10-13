#![windows_subsystem = "windows"]
use std::{error::Error, path::PathBuf, vec};
use minigrep::Config;
mod text;
use std::fs;
extern crate native_windows_gui as nwg;  // 将 `native_windows_gui` 库引入并重命名为 `nwg`
use nwg::NativeUi;
use clipboard_win::{formats,set_clipboard};
use std::path::Path;
use regex::Regex;
use zip::read::ZipArchive;
use std::io::{copy, Cursor, Read, Seek, Write};
use encoding_rs::GBK;
use flate2::read::GzDecoder;
use std::str::from_utf8;
use std::process::{Stdio,Command};
use std::rc::Rc;
use std::cell::RefCell;  // 使用 RefCell 提供内部可变性
use std::cell::Cell;

use serde::{Deserialize, Serialize};
use serde_yaml;
use dirs::home_dir;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[derive(Serialize, Deserialize)]
struct RuleConfig {
    name: String,
    enabled: bool,
    patterns: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct YamlConfig {
    rules: Vec<RuleConfig>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RuleState {
    Log,
    Package,
}
impl Default for RuleState {
    fn default() -> Self {
        RuleState::Log
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum LineState {
    Line3,
    Line1,
}
impl Default for LineState {
    fn default() -> Self {
        LineState::Line3
    }
}
impl BasicApp {
    // 重置为默认日志规则
    fn reset_to_default_log_rules(&self) {
        self.load_rules_from_text(text::LOG_RULES);
    }
    // 重置为默认发布包规则
    fn reset_to_default_package_rules(&self) {
        self.load_rules_from_text(text::PACKAGE_RULES);
    }
    // 重置默认规则根据来源
    fn load_rules_from_text(&self, rules: &[(&str, &[&str])]) {
        for (i, &(_, patterns)) in rules.iter().enumerate() {
            if i < self.features.len() {
                let feature = &self.features[i];
                feature.list_box.clear();
                for &pattern in patterns {
                    feature.list_box.push(pattern.to_string());
                }
            }
        }
    }

    // 获取配置文件路径
    fn on_generate_config_click(&self) -> Result<(), Box<dyn Error>> {
        self.save_current_config()
    }
    // 保存当前规则为配置文件
    fn save_current_config(&self) -> Result<(), Box<dyn Error>> {
        let mut rules = Vec::new();
        for feature in &self.features {
            let patterns: Vec<String> = feature.list_box.collection().to_vec();
            let enabled = feature.able_checkbox.check_state() == nwg::CheckBoxState::Checked;
            rules.push(RuleConfig {
                name: format!("规则{}", feature.id),
                enabled,
                patterns,
            });
        }
        let config = YamlConfig { rules };
        let config_content = serde_yaml::to_string(&config)?;
        fs::write(self.get_config_path(), config_content)?;
        Ok(())
    }
    // 获取配置文件内容
    fn load_config(&self) -> Result<YamlConfig, Box<dyn Error>> {
        let path = self.get_config_path();
        let config_content = fs::read_to_string(path)?;
        let config: YamlConfig = serde_yaml::from_str(&config_content)?;
        Ok(config)
    }
    // 获取配置文件路径
    fn get_config_path(&self) -> PathBuf {
        let mut config_path = home_dir().unwrap_or_else(|| PathBuf::from("."));
        config_path.push("minigrepConfig.yaml");
        config_path
    }
    // 删除配置文件
    fn confirm_delete_config(&self) -> bool {
        let params = nwg::MessageParams {
            title: "确认删除",
            content: "你确定要删除配置文件吗？",
            icons: nwg::MessageIcons::Warning,
            buttons: nwg::MessageButtons::YesNo,
        };

        let res = nwg::modal_message(&self.window, &params);
        matches!(res, nwg::MessageChoice::Yes)
    }
    
    
}

struct MatchResult {
    matched_text: String,
    file_name: String,
    line_number: String,
    origin_text: String,
}


#[derive(Default)]
pub struct  FeatureLayout {
    id: usize,
    list_box: nwg::ListBox<String>,
    input_text: nwg::TextInput,
    add_button: nwg::Button,
    save_button: nwg::Button, // 新增修改保存按钮
    remove_button: nwg::Button,
    clear_button: nwg::Button,
    able_checkbox: nwg::CheckBox,
    divider: nwg::Label,
}

impl FeatureLayout {
    pub fn initialize_defaults(&self) {
        // vec!["手机号","邮箱","身份证号","ipv4","密钥token"]
        match self.id {
            0 => {
                for pattern in text::LOG_RULES[0].1 {
                    self.list_box.push(pattern.to_string());
                }
            },
            1 => {
                for pattern in text::LOG_RULES[1].1 {
                    self.list_box.push(pattern.to_string());
                }
            },
            _ => {}  
        }
    }
}


#[derive(Default)]  // 自动为结构体实现 Default trait
pub struct BasicApp {  // 定义一个名为 BasicApp 的公共结构体
    window: nwg::Window,  // 窗口组件
    layout: nwg::GridLayout,  // 网格布局管理器

    features: Vec<FeatureLayout>,
    path_input_text: Rc<RefCell<nwg::TextInput>>,
    filedialog: nwg::FileDialog,
    browse_button: nwg::Button,
    check_button: nwg::Button,
    clear_button: nwg::Button,
    list_view: nwg::ListView,
    dyn_tis: nwg::Label,

    menu_update: nwg::MenuItem,
    menu_about: nwg::MenuItem,
    menu_generate_config: nwg::MenuItem,
    menu_delete_config_button: nwg::MenuItem,
    menu_switch_config: nwg::MenuItem,
    menu_reset_log: nwg::MenuItem,
    menu_reset_package: nwg::MenuItem,
    menu_switch_3_line: nwg::MenuItem,
    menu_switch_1_line: nwg::MenuItem,

    event_handler: RefCell<Option<nwg::EventHandler>>,
    origin_text: Rc<RefCell<nwg::RichTextBox>>,
    origin_file: Rc<RefCell<nwg::TextInput>>,
    rich_text_font: nwg::Font,

    rule_state: Cell<RuleState>,
    line_state: Rc<RefCell<LineState>>,

    ico_capoo: nwg::Icon,
}

impl BasicApp {

    // 设置规则（从配置文件）
    fn set_rules(&self,config: YamlConfig) {
        for (i, rule) in config.rules.iter().enumerate() {
            if i < self.features.len() {
                let feature = &self.features[i];
                feature.list_box.clear();
                feature.able_checkbox.set_check_state(
                    if rule.enabled {
                        nwg::CheckBoxState::Checked
                    } else {
                        nwg::CheckBoxState::Unchecked
                    }
                );
                for pattern in &rule.patterns {
                    feature.list_box.push(pattern.clone());
                }
            }
        }
        return;
    }

    fn initialize_defaults(&self) {
        self.rule_state.set(RuleState::Log);
        *self.line_state.borrow_mut() = LineState::Line1;
        // 首先检查配置文件是否存在
        let config_path = self.get_config_path();
        if config_path.exists() {
            match self.load_config() {
                Ok(config) => {
                    self.set_rules(config)
                },
                Err(e) => {
                    eprintln!("Failed to load config file: {}", e);
                }
            }
        }

        // 如果加载失败或文件不存在，则加载默认配置，因为后面已有实现，所以不管了
        

        // 写入临时cfr文件

        match fs::write("./cfr.jar", include_bytes!("cfr-0.152.jar")) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("写入cfr临时文件失败: {}", e);
                return;
            }
        }

    }

    // 处理文件拖放事件
    fn handle_file_drop(&self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
    
        let path_str = if paths.len() == 1 {
            paths[0].to_str().unwrap_or_default().to_string()
        } else {
            // 获取所有文件的公共目录
            if let Some(common_dir) = paths[0].parent() {
                common_dir.to_str().unwrap_or_default().to_string()
            } else {
                String::new()
            }
        };
    
        self.path_input_text.borrow().set_text(&path_str);
    }
    

    // 保持展示窗口列比例
    fn adjust_list_view_columns(&self) {
        let total_width = self.list_view.size().0;
        let id_col_width = (total_width as f32 * 0.10) as i32; // 10%
        let value_col_width = (total_width as f32 * 0.30) as i32; // 30%
        let file_col_width = total_width - id_col_width as u32 - value_col_width as u32; // 剩余宽度

        self.list_view.set_column_width(0, id_col_width as isize);
        self.list_view.set_column_width(1, value_col_width as isize);
        self.list_view.set_column_width(2, file_col_width as isize);
    }

    // 选择规则库列
    fn handle_list_box_select(&self, list_box_handle: &nwg::ControlHandle) {
        for feature in &self.features {
            if list_box_handle == &feature.list_box.handle {
                if let Some(selected) = feature.list_box.selection() {
                    let selected_text = feature.list_box.collection()[selected].clone();
                    feature.input_text.set_text(&selected_text);
                    feature.input_text.set_focus();
                }
            }
        }
    }

    // 修改保存规则库列
    fn save_edited_item(&self, feature_id: usize) {
        let feature = &self.features[feature_id];
        if let Some(selected) = feature.list_box.selection() {
            let edited_text = feature.input_text.text();
            let mut collection = feature.list_box.collection().clone(); // 获取并克隆当前的集合
            collection[selected] = edited_text.clone(); // 更新集合中的值
            feature.list_box.set_collection(collection); // 设置更新后的集合
            feature.input_text.set_text("");
        }
    }
    

    // 规则库按钮操作
    fn handle_button_click(&self, button_handle: &nwg::ControlHandle) {
        for feature in &self.features {
            if button_handle == &feature.add_button.handle {
                self.add_item(feature.id);
            } else if button_handle == &feature.remove_button.handle {
                self.remove_item(feature.id);
            } else if button_handle == &feature.clear_button.handle {
                self.clear_item(feature.id);
            } else if button_handle == &feature.save_button {
                self.save_edited_item(feature.id);
            }
        }
        
    }

    // 获取规则库列表
    fn get_check_regex_list(&self) -> Vec<String> {
        self.features.iter()
        .filter(|feature| feature.able_checkbox.check_state() == nwg::CheckBoxState::Checked)
        .flat_map(|feature| {
            // 假设 `collection()` 返回的是 `Ref<Vec<String>>`
            // 那么我们需要首先解引用它并克隆 `Vec` 里的数据
            let strings = feature.list_box.collection();
            strings.to_vec().into_iter()  // 这里我们将 `Ref` 中的数据克隆到一个新的 `Vec` 中
        })
        .collect()
    }

    // 获取文件进行判断
    fn get_file(&self, regex_list: Vec<String>, path: PathBuf, base_dir: &Path) -> Result<Vec<MatchResult>, Box<dyn Error>> {
        let mut all_results: Vec<MatchResult> = Vec::new(); // 用来存储所有匹配结果
        if path.is_file() {
            let file_extension = path.extension().and_then(std::ffi::OsStr::to_str).unwrap_or("");
            
            match file_extension {
                "zip" => {
                    let file = fs::File::open(&path)?;
                    all_results.extend(self.process_zip_file(&regex_list, file, &path, base_dir)?);
                },
                "war" => {
                    let file = fs::File::open(&path)?;
                    all_results.extend(self.process_war_file(&regex_list, file, &path, base_dir)?);
                },
                "jar" => {
                    let file = fs::File::open(&path)?;
                    all_results.extend(self.process_war_file(&regex_list, file, &path, base_dir)?);
                },
                "gz" => {
                    let file = fs::File::open(&path)?;
                    all_results.extend(self.process_gz_file(&regex_list, file, &path, base_dir)?);
                },
                "tar" => {
                    let file = fs::File::open(&path)?;
                    all_results.extend(self.process_tar_bytes(&regex_list, file, &path, base_dir)?);
                },
                "class" => {
                    let file = fs::File::open(&path)?;
                    all_results.extend(self.process_class_file(&regex_list, file, &path, base_dir)?);
                },
                _ => {
                    // 根据当前的 rule_state 进行文件后缀判断
                    // if self.rule_state.get() == RuleState::Package {
                    //     match file_extension {
                    //         "xml" | "properties" => { /* 继续处理 */ },
                    //         _ => return Ok(all_results), // 跳过非 xml 或 properties 文件
                    //     }
                    // }
                    let contents = match fs::read_to_string(&path) {
                        Ok(c) => c,
                        Err(_) => {
                            let contents_gbk = match fs::read(&path) {
                                Ok(bytes) => {
                                    let (cow, _, _) = GBK.decode(&bytes);
                                    cow.into_owned()
                                },
                                Err(_) => {
                                    self.dyn_tis.set_text(format!("文件 {} 不是文本文件，跳过检索", &path.to_string_lossy()).as_str());
                                    return Ok(all_results);// 如果失败，返回错误
                                }
                            };
                            contents_gbk
                        }  
                    };
                    all_results.extend(self.search_in_file_contents(&regex_list, &contents, &path, &self.strip_base_dir(base_dir, &path)));
                }
            }
        }
        Ok(all_results)
    }
    
    // 从文件夹内获取文件
    fn get_file_by_dir(&self, regex_list: Vec<String>, path_dir: PathBuf, base_dir: &Path) -> Result<Vec<MatchResult>, Box<dyn Error>> {
        let mut all_results: Vec<MatchResult> = Vec::new(); // 用来存储所有匹配结果
    
        for entry in fs::read_dir(path_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // 如果是目录，则递归调用自身
                all_results.extend(self.get_file_by_dir(regex_list.clone(), path, base_dir)?);
            } else {
                // 如果是文件，则调用 get_file 方法处理
                all_results.extend(self.get_file(regex_list.clone(), path, base_dir)?);
            }
        }
    
        Ok(all_results)
    }
    
    // 操作zip文件
    fn process_zip_file<R: Read + Seek>(&self, regex_list: &[String], reader: R, zip_path: &Path, base_dir: &Path) -> Result<Vec<MatchResult>, Box<dyn Error>> {
        let mut all_results: Vec<MatchResult> = Vec::new();
        let mut archive = ZipArchive::new(reader)?;
    
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.is_file() {
                let file_name_bytes = file.name_raw();
                let (decoded_name, _, _) = GBK.decode(file_name_bytes);
                let file_name = decoded_name.to_string();
                
                

                let mut relative_path = self.strip_base_dir(base_dir, zip_path);
                relative_path = format!("{}/{}", relative_path, file_name);  // Use relative path inside zip
    
                if file_name.ends_with(".zip") {
                    let mut nested_contents = Vec::new();
                    file.read_to_end(&mut nested_contents)?;
                    let cursor = Cursor::new(nested_contents);
                    all_results.extend(self.process_zip_file(&regex_list, cursor, Path::new(&relative_path), base_dir)?);
                } else if file_name.ends_with(".gz") {
                    let mut nested_contents = Vec::new();
                    file.read_to_end(&mut nested_contents)?;
                    let cursor = Cursor::new(nested_contents);
                    all_results.extend(self.process_gz_file(&regex_list, cursor, Path::new(&relative_path), base_dir)?);
                } else if file_name.ends_with(".tar") {
                    let mut nested_contents = Vec::new();
                    file.read_to_end(&mut nested_contents)?;
                    let cursor = Cursor::new(nested_contents);
                    all_results.extend(self.process_tar_bytes(&regex_list, cursor, Path::new(&relative_path), base_dir)?);
                } else if file_name.ends_with(".war") {
                    let mut nested_contents = Vec::new();
                    file.read_to_end(&mut nested_contents)?;
                    let cursor = Cursor::new(nested_contents);
                    all_results.extend(self.process_war_file(&regex_list, cursor, Path::new(&relative_path), base_dir)?);
                } else if file_name.ends_with(".jar") {
                    let mut nested_contents = Vec::new();
                    file.read_to_end(&mut nested_contents)?;
                    let cursor = Cursor::new(nested_contents);
                    all_results.extend(self.process_war_file(&regex_list, cursor, Path::new(&relative_path), base_dir)?);
                } else if file_name.ends_with(".class") {
                    let mut nested_contents = Vec::new();
                    file.read_to_end(&mut nested_contents)?;
                    let cursor = Cursor::new(nested_contents);
                    all_results.extend(self.process_class_file(&regex_list, cursor, Path::new(&relative_path), base_dir)?);
                } else {
                    // 如果是发布包状态且文件不符合要求，跳过
                    // if self.rule_state.get() == RuleState::Package {
                    //     if !file_name.ends_with(".xml") && !file_name.ends_with(".properties") {
                    //         continue;
                    //     }
                    // }

                    let mut contents = Vec::new();
                    let contents_str = match file.read_to_end(&mut contents) {
                        Ok(_) => match String::from_utf8(contents.clone()) {
                            Ok(c) => c,
                            Err(_) => {
                                let (cow, _, had_errors) = GBK.decode(&contents);
                                if had_errors {
                                    self.dyn_tis.set_text(format!("文件 {} 不是文本文件，跳过检索", &relative_path).as_str());
                                    continue; // 跳过此文件
                                }
                                cow.into_owned()
                            }
                        },
                        Err(_) => {
                            self.dyn_tis.set_text(format!("文件 {} 不是文本文件，跳过检索", &relative_path).as_str());
                            continue; // 跳过此文件
                        }
                    };
                    all_results.extend(self.search_in_file_contents(&regex_list, &contents_str, Path::new(&relative_path), &relative_path));
                }
            }
        }
    
        Ok(all_results)
    }
    
    // 操作gz文件
    fn process_gz_file<R: Read>(&self, regex_list: &[String], reader: R, gz_path: &Path, base_dir: &Path) -> Result<Vec<MatchResult>, Box<dyn Error>> {
        let mut all_results: Vec<MatchResult> = Vec::new();
        let mut decoder = GzDecoder::new(reader);
        let mut decompressed_data = Vec::new();
        match decoder.read_to_end(&mut decompressed_data) {
            Ok(_) => (),
            Err(e) => {
                self.dyn_tis.set_text(format!("解压文件 {} 失败: {}", gz_path.to_string_lossy(), e).as_str());
                return Ok(all_results);
            }
        }
        // 假设.gz文件可能是.tar.gz
        if gz_path.file_name().and_then(|name| name.to_str()).map_or(false, |name| name.ends_with(".tar.gz")) {
            let cursor = Cursor::new(&decompressed_data);
            return self.process_tar_bytes(regex_list, cursor, gz_path, base_dir);
        }
        let cursor = Cursor::new(&decompressed_data);
        // 进一步检查解压后的文件类型
        let archive = ZipArchive::new(cursor.clone());
        if archive.is_ok() {
            return self.process_zip_file(regex_list, cursor, gz_path, base_dir);
        }

        let cursor = Cursor::new(decompressed_data.clone());
        let mut decoder = GzDecoder::new(cursor);
        let mut nested_decompressed_data = Vec::new();
        if decoder.read_to_end(&mut nested_decompressed_data).is_ok() {
            let nested_cursor = Cursor::new(nested_decompressed_data);
            return self.process_gz_file(regex_list, nested_cursor, gz_path, base_dir);
        }

        // 进一步处理解压后的文件
        // if self.rule_state.get() == RuleState::Package {
        //     if !gz_path.ends_with(".xml") && !gz_path.ends_with(".properties") {
        //         return Ok(all_results);
        //     }
        // }
    
        let contents_str = match String::from_utf8(decompressed_data.clone()) {
            Ok(c) => c,
            Err(_) => {
                let (cow, _, had_errors) = GBK.decode(&decompressed_data);
                if had_errors {
                    self.dyn_tis.set_text(format!("文件 {} 不是文本文件，跳过检索", gz_path.to_string_lossy()).as_str());
                    return Ok(all_results);
                }
                cow.into_owned()
            }
        };
    
        let relative_path = self.strip_base_dir(base_dir, gz_path);
        all_results.extend(self.search_in_file_contents(&regex_list, &contents_str, gz_path, &relative_path));
    
        Ok(all_results)
    }
    
    // 操作tar文件
    fn process_tar_bytes<R: Read>(&self, regex_list: &[String], mut reader: R, tar_path: &Path, base_dir: &Path) -> Result<Vec<MatchResult>, Box<dyn Error>> {
        let mut all_results: Vec<MatchResult> = Vec::new();
        let mut buffer = [0; 512];
    
        loop {
            if reader.read_exact(&mut buffer).is_err() {
                break;
            }
    
            let file_name = match from_utf8(&buffer[0..100]) {
                Ok(name) => name.trim_matches(char::from(0)).to_string(),
                Err(_) => break,
            };
    
            if file_name.is_empty() {
                break;
            }

            

            let size_str = match from_utf8(&buffer[124..136]) {
                Ok(size) => size.trim_matches(char::from(0)),
                Err(_) => break,
            };
    
            let size = usize::from_str_radix(size_str, 8).unwrap_or(0);
    
            let mut contents = vec![0; size];
            reader.read_exact(&mut contents)?;
    
            let relative_path = format!("{}/{}", self.strip_base_dir(base_dir, tar_path), file_name);
    
            if file_name.ends_with(".tar") {
                let cursor = Cursor::new(contents);
                all_results.extend(self.process_tar_bytes(&regex_list, cursor, Path::new(&relative_path), base_dir)?);
            } else if file_name.ends_with(".gz") {
                let cursor = Cursor::new(contents);
                all_results.extend(self.process_gz_file(&regex_list, cursor, Path::new(&relative_path), base_dir)?);
            } else if file_name.ends_with(".zip") {
                let cursor = Cursor::new(contents);
                all_results.extend(self.process_zip_file(&regex_list, cursor, Path::new(&relative_path), base_dir)?);
            } else if file_name.ends_with(".war") {
                let cursor = Cursor::new(contents);
                all_results.extend(self.process_war_file(&regex_list, cursor, Path::new(&relative_path), base_dir)?);
            } else if file_name.ends_with(".jar") {
                let cursor = Cursor::new(contents);
                all_results.extend(self.process_war_file(&regex_list, cursor, Path::new(&relative_path), base_dir)?);
            } else if file_name.ends_with(".class") {
                let cursor = Cursor::new(contents);
                all_results.extend(self.process_class_file(&regex_list, cursor, Path::new(&relative_path), base_dir)?);
            } else {
                // 如果是发布包状态且文件不符合要求，跳过
                // if self.rule_state.get() == RuleState::Package {
                //     if !file_name.ends_with(".xml") && !file_name.ends_with(".properties") {
                //         continue;
                //     }
                // }
                let contents_str = match String::from_utf8(contents.clone()) {
                    Ok(c) => c,
                    Err(_) => {
                        let (cow, _, had_errors) = GBK.decode(&contents);
                        if had_errors {
                            self.dyn_tis.set_text(format!("文件 {} 不是文本文件，跳过检索", tar_path.to_string_lossy()).as_str());
                            continue;
                        }
                        cow.into_owned()
                    }
                };
                all_results.extend(self.search_in_file_contents(&regex_list, &contents_str, Path::new(&relative_path), &relative_path));
            }
    
            let remainder = 512 - (size % 512);
            if remainder < 512 {
                let mut skip = vec![0; remainder];
                reader.read_exact(&mut skip)?;
            }
        }
    
        Ok(all_results)
    }
    
    // 操作war文件 jar也是
    fn process_war_file<R: Read + Seek>(&self, regex_list: &[String], reader: R, war_path: &Path, base_dir: &Path) -> Result<Vec<MatchResult>, Box<dyn Error>> {
        // WAR 文件本质上是 ZIP 文件，所以我们可以调用 process_zip_file
        self.process_zip_file(regex_list, reader, war_path, base_dir)
    }

    // 操作class文件
    fn process_class_file<R: Read>(&self, regex_list: &[String],mut reader: R, class_path: &Path, base_dir: &Path) -> Result<Vec<MatchResult>, Box<dyn Error>> {

        let mut all_results: Vec<MatchResult> = Vec::new();
        let relative_path = self.strip_base_dir(base_dir, class_path);

        // 手动创建临时文件路径
        let temp_dir = std::env::temp_dir();
        let temp_file_path = temp_dir.join("temp_class_file.class");

        // 将 class 文件数据写入临时文件
        {
            let mut temp_file = fs::File::create(&temp_file_path)?;
            std::io::copy(&mut reader, &mut temp_file)?;
        }

            let mut command = Command::new("java");
                command.arg("-jar")
                    .arg("./cfr.jar")
                    .arg(&temp_file_path) // 使用 "-" 表示从标准输入读取内容
                    .stdout(Stdio::piped())  // 将标准输出重定向到管道
                    .stderr(Stdio::piped())  // 将标准错误重定向到管道
                    .stdin(Stdio::null());
        

        #[cfg(target_os = "windows")]
        {
            command.creation_flags(0x08000000); // Windows 特定：创建隐藏窗口（仅在 Windows 平台上编译时有效）
        }

        // 运行命令并获取输出
        let output = command.spawn()
            .and_then(|child| child.wait_with_output());

        // 删除临时文件
        std::fs::remove_file(&temp_file_path)?;

        match output {
            Ok(output) => {
                if output.status.success() {
                    if !output.stdout.is_empty() {
                        let result = String::from_utf8_lossy(&output.stdout);
                        all_results.extend(self.search_in_file_contents(&regex_list, &result, class_path, &relative_path));
                    } else {
                        self.dyn_tis.set_text(format!("反编译失败：{} {}", class_path.to_string_lossy(), base_dir.to_string_lossy()).as_str());
                    }
                } else {
                    self.dyn_tis.set_text(format!("反编译失败：{} {}", class_path.to_string_lossy(), String::from_utf8_lossy(&output.stderr)).as_str());
                }
            }
            Err(e) => {
                self.dyn_tis.set_text(format!("反编译失败：{}", e).as_str());
            }
        }

        
        Ok(all_results)
    }

    // 搜索文件内容
    fn search_in_file_contents(&self, regex_list: &[String], contents: &str, path: &Path, file_name: &str) -> Vec<MatchResult> {
        let mut results = Vec::new();
        for query in regex_list {
            let mut query_regex = query.clone();
            // 这里替换了发布包扫描默认规则库第一条，对于class代码扫描时强制启用引号检测
            if file_name.ends_with(".class"){
                if query == r#"((P|p)((A|a)(S|s)(S|s))?(W|w)((O|o)(R|r))?(D|d)|(K|k)(E|e)(Y|y)|(E|e)(N|n)(C|c)(R|r)(Y|y)(P|p)(T|t)|(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)|(A|a)(U|u)(T|t)(H|h)((O|o)(R|r)(I|i)(Z|z)(A|a)(T|t)(I|i)(O|o)(N|n))?)\s?[\"\']?(=|:)+\s?[\"\']?[a-zA-Z0-9\@\.]+[\"\']?"# {
                    query_regex = String::from(r#"((P|p)((A|a)(S|s)(S|s))?(W|w)((O|o)(R|r))?(D|d)|(K|k)(E|e)(Y|y)|(E|e)(N|n)(C|c)(R|r)(Y|y)(P|p)(T|t)|(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)|(A|a)(U|u)(T|t)(H|h)((O|o)(R|r)(I|i)(Z|z)(A|a)(T|t)(I|i)(O|o)(N|n))?)\s?[\"\']?(=|:)+\s?[\"\']+[a-zA-Z0-9\@\.]+[\"\']+"#);
                }
                
            }
            let config = Config {
                query: query_regex,
                contents: contents.to_string(),
                ignore_case: false,
            };
            if let Ok(matches) = minigrep::run(config) {
                for (line_number, matched_text, origin_text) in matches {
                    let display_file_name = if path.is_file() {
                        file_name.to_string()  // 直接使用文件名，不需要额外的路径信息
                    } else {
                        format!("{}", file_name)
                    };
                    if *self.line_state.borrow() == LineState::Line3{
                        results.push(MatchResult {
                            file_name: display_file_name,
                            line_number,
                            matched_text,
                            origin_text
                            }   
                        );
                    } else {
                        let lines: Vec<&str> = origin_text.split('\n').collect();
                        results.push(MatchResult {
                            file_name: display_file_name,
                            line_number,
                            matched_text,
                            origin_text:lines[1].to_string(),
                            }   
                        );
                    }}
            }
        }
        results
    }
    
    // 获取目录下所有文件
    fn get_all_file(&self, regex_list: Vec<String>, path_dir: String) -> Result<Vec<MatchResult>, Box<dyn Error>> {
            // let res = self.is_dir_or_file(regex_list, path_dir);
            let path = PathBuf::from(path_dir);
            let all_results: Result<Vec<MatchResult>, Box<dyn Error>>;
            let base_dir = path.clone();
            if path.is_dir() {
                // 如果是目录，则递归处理目录中的所有文件
                all_results = self.get_file_by_dir(regex_list, path, &base_dir)
            } else {
                // 如果是文件，直接处理该文件
                all_results = self.get_file(regex_list, path, &base_dir)
            }
            all_results
    }

    
    // 检测按钮点击后
    fn begin_check(&self) {
        self.list_view.clear();
        let directory = self.path_input_text.borrow().text();
        if directory.is_empty() {
            self.path_input_text.borrow().set_text("请输入日志目录");
            return;
        }
        self.dyn_tis.set_text("搜索中...");

        

        let regex_list: Vec<String> = self.get_check_regex_list();
        let all_results = self.get_all_file(regex_list, directory);
        match all_results {
            Ok(all_res) => {
                // 用于临时保存所有的完整文本和匹配文本
                let mut matched_text_storage: Vec<String> = Vec::new();  // 新增
                let mut full_text_storage: Vec<String> = Vec::new();
                let mut file_name_storage: Vec<String> = Vec::new();
                for result in all_res {
                    let list_view_num = self.list_view.len();
                    
                    self.list_view.insert_item(nwg::InsertListViewItem {
                        column_index: 0,
                        text: Some(list_view_num.to_string()),
                        index: Some(list_view_num as i32),
                        image: None,
                    });
    
                    self.list_view.insert_item(nwg::InsertListViewItem {
                        column_index: 1,
                        text: Some(result.matched_text.clone()),
                        index: Some(list_view_num as i32),
                        image: None,
                    });


                    self.list_view.insert_item(nwg::InsertListViewItem {
                        column_index: 2,
                        text: Some(format!("{} 第 {} 行", result.file_name, result.line_number)),
                        index: Some(list_view_num as i32),
                        image: None,
                    });
    
                    // 保存完整的 origin_text 和 matched_text 到临时存储中
                    full_text_storage.push(result.origin_text.clone());
                    file_name_storage.push(format!("{} 第 {} 行", result.file_name, result.line_number));
                    matched_text_storage.push(result.matched_text.clone());  // 新增
                    
                }
                // 将完整文本存储到 `ListView` 的 `userdata` 中
                self.bind_copy_event(full_text_storage,file_name_storage, matched_text_storage);
            },
            _ => { self.dyn_tis.set_text("该目录或文件中有文件内容为非文本内容，筛查失败，请检查后再试") }
        }
        if self.dyn_tis.text() == "搜索中..." {
            self.dyn_tis.set_text("搜索完成");
        }
    }
    
    fn bind_copy_event(&self, full_text_storage: Vec<String>,file_names: Vec<String>, matched_texts: Vec<String>) {
        let copy_storage = Rc::new(full_text_storage);
        let file_name_storage = Rc::new(file_names);
        let matched_text_storage = Rc::new(matched_texts);  // 新增
        // 解除之前的事件处理器
        if let Some(handler) = self.event_handler.borrow_mut().take() {
            nwg::unbind_event_handler(&handler);
        }
        // 绑定 `ListView` 的激活事件来处理复制逻辑
        let list_view_handle = &self.list_view.handle;
        let window_handle = &self.window.handle;
        let origin_text = Rc::clone(&self.origin_text);
        let origin_file = Rc::clone(&self.origin_file);
        let path_input_text = Rc::clone(&self.path_input_text);
        let new_handler = nwg::bind_event_handler(
            list_view_handle,  // 控件句柄
            window_handle,  // 父窗口句柄
            {
                let copy_storage = Rc::clone(&copy_storage);
                let file_name_storage = Rc::clone(&file_name_storage);
                let matched_text_storage = Rc::clone(&matched_text_storage);  // 新增
                let path_input_text = Rc::clone(&path_input_text);
                // 正则表达式用于匹配 Unicode 转义字符
                let re = Regex::new(r"\\u([0-9a-fA-F]{4})").unwrap();
                move |evt, evt_data, _handle| {
                    match evt {
                        nwg::Event::OnListViewClick => {
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
                            
                        },
                        nwg::Event::OnFileDrop => {
                            let paths: Vec<PathBuf> = evt_data.on_file_drop().files().into_iter().map(PathBuf::from).collect();
                            if paths.is_empty() {
                                return;
                            }
                        
                            let path_str = if paths.len() == 1 {
                                paths[0].to_str().unwrap_or_default().to_string()
                            } else {
                                // 获取所有文件的公共目录
                                if let Some(common_dir) = paths[0].parent() {
                                    common_dir.to_str().unwrap_or_default().to_string()
                                } else {
                                    String::new()
                                }
                            };
                        
                            path_input_text.borrow().set_text(&path_str);
                            
                        },
                        _ => {}
                    }
                }
            }
        );
        // 存储新的事件处理器
        *self.event_handler.borrow_mut() = Some(new_handler);
    }

    // 清空展示列表
    fn clear_list_view(&self) {
        self.list_view.clear();
    }

    // 添加按钮添加规则实例
    fn add_item(&self, feature_id: usize) {
        let feature = &self.features[feature_id];
        let text = feature.input_text.text();
        if !text.is_empty() {
            feature.list_box.push(text);
            feature.input_text.set_text("");
        }
    }

    // 删除按钮删除规则实例
    fn remove_item(&self, feature_id: usize) {
        let feature = &self.features[feature_id];
        if let Some(selected) = feature.list_box.selection() {
            feature.list_box.remove(selected);
        }
    }

    // 清除按钮清空规则库示例
    fn clear_item(&self, feature_id: usize) {
        let feature = &self.features[feature_id];
        feature.list_box.clear();
    }

    // 打开资源管理器载入检索路径
    fn open_file_dialog(&self, handle: &nwg::ControlHandle) {
        let mut c = String::new();
        if self.filedialog.run(Some(handle)) {
            match self.filedialog.get_selected_items() {
                Ok(res) => {
                    if res.len() < 1 {
                       
                    } else if res.len() == 1 {
                        c.push_str(&res[0].to_str().unwrap());
                    } else {
                        let file_path = &res[0].to_str().unwrap();
                        let path = Path::new(file_path);
                        // 获取目录部分
                        if let Some(parent) = path.parent() {
                            // parent 是一个 Option<&std::path::Path>
                            // 使用 .to_string_lossy() 方法转换为一个 String
                            let directory = parent.to_string_lossy().to_string();
                            // 现在 directory 就是所在目录的字符串表示
                            c.push_str(directory.as_str());
                        }
                    }
                },
                _ => {}
            };
            if c.len() > 1 {
                self.path_input_text.borrow().set_text(c.as_str());
            }
        }
    }
    
    
    // 复制参数
    fn match_copy(&self,_handle: &nwg::ControlHandle) {
        if let Some(index) = self.list_view.selected_item() {
            if let Some(item1) = self.list_view.item(index,1,100) {
                if let Some(_item2) = self.list_view.item(index,2,100) {
                    // let text = item1.text + " " + item2.text.as_str();
                    if let Err(_e) = set_clipboard(formats::Unicode, item1.text.clone()){

                    } else {
                        self.dyn_tis.set_text(format!("已复制内容: {}",&item1.text.to_string()).as_str());
                    };
                }
            }
        }
        
        
    }



    // 绝对路径变相对路径
    fn strip_base_dir(&self, base_dir: &Path, full_path: &Path) -> String {
        full_path.strip_prefix(base_dir)
            .unwrap_or(full_path)
            .to_string_lossy()
            .to_string()
    }
}
    

mod basic_app_ui {  // 定义一个模块，用于用户界面的管理
    use super::*; 
    // 引入上级作用域中的所有项
    use std::rc::Rc;  // 使用 Rc 用于引用计数的智能指针
    use std::cell::RefCell;  // 使用 RefCell 提供内部可变性
    use std::ops::Deref;  
    // 引入 Deref trait 用于自定义解引用行为
    use nwg::CheckBoxState;

    pub struct BasicAppUi {  // 定义 UI 管理结构体
        inner: Rc<BasicApp>,  // 使用 Rc 封装 BasicApp，允许多处共享所有权
        default_handler: RefCell<Option<nwg::EventHandler>>  // 事件处理器，用 RefCell 提供内部可变性
    }

    impl nwg::NativeUi<BasicAppUi> for BasicApp {  // 实现 NativeUi trait 用于构建 UI
        fn build_ui(mut data: BasicApp) -> Result<BasicAppUi, nwg::NwgError> {  // 构建 UI 并返回 BasicAppUi 或错误
            use nwg::Event as E;  // 引入 Event 枚举简化名称
            
            let icon_data = include_bytes!("../capoo.ico");
            let icon = nwg::Icon::from_bin(icon_data).expect("Failed to load icon from bytes");
            // 创建窗口
            nwg::Window::builder()
                .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE | nwg::WindowFlags::MINIMIZE_BOX | nwg::WindowFlags::MAXIMIZE_BOX | nwg::WindowFlags::RESIZABLE)  // 窗口属性
                .size((1100, 800))  // 窗口大小
                .position((150, 80))  // 窗口位置
                .accept_files(true)
                .icon(Some(&icon))
                .title(text::TITLE)  // 窗口标题
                //.maximized(true)
                .build(&mut data.window)?;  // 构建窗口并处理错误

            // 设置任务栏图标
            let _ = nwg::Icon::builder()
                .source_bin(Some(icon_data))
                .strict(true)
                .build(&mut data.ico_capoo);
            data.window.set_icon(Some(&data.ico_capoo));
            
            // 初始化菜单和菜单项
            nwg::MenuItem::builder()
                .text("更新日志")
                .parent(&data.window)
                .build(&mut data.menu_update)?;

            nwg::MenuItem::builder()
                .text("关于|注意事项")
                .parent(&data.window)
                .build(&mut data.menu_about)?;

            nwg::MenuItem::builder()
                .text("生成/更新配置文件")
                .parent(&data.window)
                .build(&mut data.menu_generate_config)?;

            nwg::MenuItem::builder()
                .text("删除配置文件")
                .parent(&data.window)
                .build(&mut data.menu_delete_config_button)?;
            
            nwg::MenuItem::builder()
                .text("切换到配置文件")
                .parent(&data.window)
                .build(&mut data.menu_switch_config)?;
        
            nwg::MenuItem::builder()
                .text("重置为默认日志规则")
                .parent(&data.window)
                .build(&mut data.menu_reset_log)?;

            nwg::MenuItem::builder()
                .text("重置为默认发布包规则")
                .parent(&data.window)
                .build(&mut data.menu_reset_package)?;

            nwg::MenuItem::builder()
                .text("切换到3行展示")
                .parent(&data.window)
                .build(&mut data.menu_switch_3_line)?;

            nwg::MenuItem::builder()
                .text("切换到1行展示")
                .parent(&data.window)
                .build(&mut data.menu_switch_1_line)?;

            // 添加输入框和按钮
            nwg::TextInput::builder()
                .parent(&data.window)
                .text("此处粘贴日志目录")  // 初始文本为空
                .build(&mut data.path_input_text.borrow_mut())?;

            nwg::Button::builder()
                .text("选择文件/文件夹")
                .parent(&data.window)
                .build(&mut data.browse_button)?;

            nwg::Button::builder()
                .text("开始检测")  // 根据需要设定按钮功能
                .parent(&data.window)
                .build(&mut data.check_button)?;
            
            nwg::Button::builder()
                .text("清空检测")  // 根据需要设定按钮功能
                .parent(&data.window)
                .build(&mut data.clear_button)?;

            // 原文展示框
            
            nwg::Font::builder()
                .family("黑体") // 您可以根据需要选择字体，如 "Segoe UI"、"宋体" 等
                .size(20)          // 设置字体大小为 14，根据需要调整
                .build(&mut data.rich_text_font)?;

            data.origin_text = Rc::new(RefCell::new(nwg::RichTextBox::default()));
            nwg::RichTextBox::builder()
                .parent(&data.window)
                .text("此处展示上下三行时，值所在行为中间那一行")  // 初始文本为空
                .font(Some(&data.rich_text_font))
                .build(&mut data.origin_text.borrow_mut())?;
            data.origin_text.borrow_mut().set_background_color([155, 200, 200]);
            
            data.origin_file = Rc::new(RefCell::new(nwg::TextInput::default()));
            nwg::TextInput::builder()
                .parent(&data.window)
                .text("此处展示来源名")  // 初始文本为空
                .build(&mut data.origin_file.borrow_mut())?;

            nwg::ListView::builder()
                .parent(&data.window)
                .item_count(5)
                .list_style(nwg::ListViewStyle::Detailed)  // 设置为报表样式，支持列标题
                .build(&mut data.list_view)
                .expect("Failed to create list view");

            nwg::Label::builder()
                .parent(&data.window)
                .text("这里是提示框")
                .build(&mut data.dyn_tis)
                .expect("动态文字展示出错");

            let _ = nwg::FileDialog::builder()
                .title("Hello")
                .action(nwg::FileDialogAction::Open)
                .multiselect(true)
                .build(&mut data.filedialog);

            // 使用 InsertListViewColumn 来添加列
            data.list_view.set_headers_enabled(true);

            let list_view_col = vec!["id","值","文件及所在行"].into_iter().enumerate().map(|(index,col_name)|(index as i32, col_name));

            for (index,col_name) in list_view_col {
                
                if index == 0 {
                    data.list_view.insert_column(nwg::InsertListViewColumn {
                        index: Some(index), // 列的位置
                        text: Some(col_name.to_string()), // 列标题
                        width: Some(62),
                        
                        ..Default::default()
                    });
                } else if index == 1 {
                    data.list_view.insert_column(nwg::InsertListViewColumn {
                        index: Some(index), // 列的位置
                        text: Some(col_name.to_string()), // 列标题
                        width: Some(152),
                        
                        ..Default::default()
                    });
                } else if index == 2 {
                    data.list_view.insert_column(nwg::InsertListViewColumn {
                        index: Some(index), // 列的位置
                        text: Some(col_name.to_string()), // 列标题
                        width: Some(312),
                        
                        ..Default::default()
                    });
                }
                
            };
            data.list_view.enabled();



            let layout_list = vec!["规则匹配","关键字匹配"].into_iter();
            data.features = layout_list.enumerate().map(|(index,_name)| FeatureLayout {
                id:index ,
                list_box: Default::default(),
                input_text: Default::default(),
                add_button: Default::default(),
                save_button: Default::default(),
                remove_button: Default::default(),
                clear_button: Default::default(),
                able_checkbox: Default::default(),
                // regex_checkbox: Default::default(),
                divider: Default::default(),
            }).collect::<Vec<_>>();
            let layout_list = vec!["规则匹配","关键字匹配"].into_iter();
            for (index,x) in layout_list.enumerate() {
                // data.features[index].name = x.clone();

                nwg::ListBox::builder()
                .parent(&data.window)
                .build(&mut data.features[index].list_box)?;

                nwg::TextInput::builder()
                    .parent(&data.window)
                    .build(&mut data.features[index].input_text)?;

                nwg::Button::builder()
                    .text("添加")
                    .parent(&data.window)
                    .build(&mut data.features[index].add_button)?;

                nwg::Button::builder()
                    .text("修改保存")
                    .parent(&data.window)
                    .build(&mut data.features[index].save_button)?;

                // Remove button setup
                nwg::Button::builder()
                    .text("删除")
                    .parent(&data.window)
                    .build(&mut data.features[index].remove_button)?;

                // Remove button setup
                nwg::Button::builder()
                    .text("清空")
                    .parent(&data.window)
                    .build(&mut data.features[index].clear_button)?;

                // Checkbox setup
                nwg::CheckBox::builder()
                    .text(&format!("启用{}", x))
                    .parent(&data.window)
                    .check_state(CheckBoxState::Checked)
                    .build(&mut data.features[index].able_checkbox)?;

                nwg::Label::builder()  // 使用 Label 作为分割线
                    .text("")
                    .parent(&data.window)
                    .size((20, 1))  // 设定分割线尺寸
                    .background_color(Some([200, 200, 200]))
                    .build(&mut data.features[index].divider)?;
            }

            

            data.event_handler = RefCell::new(None);
            // Event handling
            let ui = BasicAppUi {
                inner: Rc::new(data),
                default_handler: Default::default(),
            };
            
            // 事件绑定
            let evt_ui = Rc::downgrade(&ui.inner);
            let handle_events = move |evt, _evt_data:nwg::EventData, handle| {
                if let Some(ui) = evt_ui.upgrade() {
                    match evt {
                        E::OnButtonClick => {
                            if &handle == &ui.browse_button.handle {
                                ui.open_file_dialog(&handle);
                            } else if &handle == &ui.check_button {
                                ui.begin_check();
                            } else if &handle == &ui.clear_button {
                                ui.clear_list_view();
                            } else {
                                ui.handle_button_click(&handle);
                            }// 处理保存按钮点击
                        },
                        E::OnWindowClose => {
                            // 清理临时文件
                            if let Err(e) = fs::remove_file("./cfr.jar") {
                                eprintln!("未找到cfr临时文件: {}", e);
                            }
                            std::process::exit(1);
                        },
                        E::OnListViewRightClick => ui.match_copy(&handle),
                        E::OnMenuItemSelected => {
                            if &handle == &ui.menu_about { // 关于按钮
                                nwg::simple_message("关于&注意事项", text::ABOUT_TEXT);
                            } else if &handle == &ui.menu_update { // 更新日志
                                nwg::simple_message("更新日志", text::UPDATE_LOG);
                            } else if &handle == &ui.menu_generate_config {  // 生成日志文件
                                if let Err(e) = ui.on_generate_config_click() {
                                    nwg::simple_message("错误", &format!("生成配置文件失败: {}", e));
                                } else {
                                    nwg::simple_message("成功", "配置文件已生成并更新");
                                }
                            }else if &handle == &ui.menu_delete_config_button {  // 删除配置文件菜单按钮
                                let config_path = ui.get_config_path();
                                if config_path.exists() {
                                    if ui.confirm_delete_config() {  // 显示确认弹窗
                                        if let Err(e) = fs::remove_file(&config_path) {
                                            nwg::simple_message("错误", &format!("删除配置文件失败: {}", e));
                                        } else {
                                            nwg::simple_message("成功", "配置文件已删除");
                                        }
                                    }
                                } else {
                                    nwg::simple_message("错误", "配置文件不存在");
                                }
                            } else if &handle == &ui.menu_switch_config { // 切换为配置文件日志规则
                                for feature in &ui.features {
                                    feature.list_box.clear();
                                }
                                let res = ui.load_config();
                                match res {
                                    Ok(config) => {
                                        ui.set_rules(config);
                                    },
                                    Err(e) => {
                                        nwg::simple_message("错误", format!("配置文件可能不存在：{}",e).as_str());
                                    }
                                } 
                                ui.rule_state.set(RuleState::Log);
                                *ui.line_state.borrow_mut() = LineState::Line1;
                                ui.dyn_tis.set_text("切换到日志规则库，默认显示1行匹配值，下次搜索时生效")
                                
                            } else if &handle == &ui.menu_reset_log { // 重置为默认日志规则
                                for feature in &ui.features {
                                    feature.list_box.clear();
                                }
                                ui.reset_to_default_log_rules(); 
                                ui.rule_state.set(RuleState::Log);
                                *ui.line_state.borrow_mut() = LineState::Line1;
                                ui.dyn_tis.set_text("切换到日志规则库，默认显示1行匹配值，下次搜索时生效")
                            } else if &handle == &ui.menu_reset_package {// 重置为默认发布包规则
                                for feature in &ui.features {
                                    feature.list_box.clear();
                                }
                                ui.reset_to_default_package_rules();
                                ui.rule_state.set(RuleState::Package);
                                *ui.line_state.borrow_mut() = LineState::Line3;
                                // 因为发布版规则匹配比较完善，所以默认取消关键字匹配了
                                ui.features[1].able_checkbox.set_check_state(nwg::CheckBoxState::Unchecked);
                                ui.dyn_tis.set_text("切换到发布包规则库，默认显示3行匹配值，下次搜索时生效")
                            } else if &handle == &ui.menu_switch_3_line {
                                *ui.line_state.borrow_mut() = LineState::Line3;
                                ui.dyn_tis.set_text("切换到3行匹配值，下次搜索时生效");
                            } else if &handle == &ui.menu_switch_1_line {
                                *ui.line_state.borrow_mut() = LineState::Line1;
                                ui.dyn_tis.set_text("切换到1行匹配值，下次搜索时生效");
                            }
                        },
                        E::OnListBoxSelect => ui.handle_list_box_select(&handle),
                        E::OnResize => {
                            ui.adjust_list_view_columns();
                        },
                        E::OnFileDrop => {
                            let files = _evt_data.on_file_drop().files().into_iter().map(PathBuf::from).collect();
                            if let Some(ui) = evt_ui.upgrade() {
                                ui.handle_file_drop(files);
                            };
                        },
                        _ => {}
                    }
                }
            };

            

           *ui.default_handler.borrow_mut() = Some(nwg::full_bind_event_handler(&ui.window.handle, handle_events));

           // 设置布局
            let mut tmp = nwg::GridLayout::builder()
                .parent(&ui.window)
                .spacing(1);
            let mut row_num = 0;
            let mut col_num = 0;
            let mut col_index = 0;
            for x in &ui.features {
                if col_index > 2 {
                    col_num += 2;
                    row_num = 0;
                    col_index = 0;
                } else {

                }

                tmp = tmp.child_item(nwg::GridLayoutItem::new(&x.list_box, col_num, row_num, 1, 6))
                    .child_item(nwg::GridLayoutItem::new(&x.able_checkbox, col_num + 1 , row_num, 1, 1))
                    // .child_item(nwg::GridLayoutItem::new(&x.regex_checkbox, col_num + 1, row_num + 1, 1, 1))
                    .child_item(nwg::GridLayoutItem::new(&x.input_text, col_num + 1, row_num + 1, 1, 1))
                    .child_item(nwg::GridLayoutItem::new(&x.add_button, col_num + 1, row_num + 2, 1, 1))
                    .child_item(nwg::GridLayoutItem::new(&x.save_button, col_num + 1, row_num + 3, 1, 1)) // 新增保存按钮布局
                    .child_item(nwg::GridLayoutItem::new(&x.remove_button, col_num + 1, row_num + 4, 1, 1))
                    .child_item(nwg::GridLayoutItem::new(&x.clear_button, col_num + 1, row_num + 5, 1, 1))
                    .child_item(nwg::GridLayoutItem::new(&x.divider, col_num, row_num + 6, 2, 1));
                row_num += 7;
                col_index += 1;
                x.initialize_defaults();
            
            }
            tmp = tmp.child_item(nwg::GridLayoutItem::new(&ui.dyn_tis, col_num, row_num-1 , 2, 2))
                .child_item(nwg::GridLayoutItem::new(&ui.path_input_text.borrow().handle, col_num, row_num + 1, 1, 1))
                .child_item(nwg::GridLayoutItem::new(&ui.browse_button, col_num +1 , row_num + 1, 1, 1))
                .child_item(nwg::GridLayoutItem::new(&ui.check_button, col_num , row_num + 2, 1, 1))
                .child_item(nwg::GridLayoutItem::new(&ui.clear_button, col_num + 1 , row_num + 2, 1, 1))
                .child_item(nwg::GridLayoutItem::new(&ui.origin_text.borrow().handle, col_num + 2, 0, 2, 3))
                .child_item(nwg::GridLayoutItem::new(&ui.origin_file.borrow().handle, col_num + 2, 3, 2, 1))
                .child_item(nwg::GridLayoutItem::new(&ui.list_view, col_num + 2 , 4, 2, 13));

            ui.inner.initialize_defaults();

            tmp.build(&ui.layout)?;

            return Ok(ui);
        }

        
    }

    impl Drop for BasicAppUi {  // 实现 Drop trait 以确保资源正确释放
        fn drop(&mut self) {
            let handler = self.default_handler.borrow();
            if handler.is_some() {
                nwg::unbind_event_handler(handler.as_ref().unwrap());
            }
        }
    }

    impl Deref for BasicAppUi {  // 实现 Deref trait 提供解引用功能
        type Target = BasicApp;

        fn deref(&self) -> &BasicApp {
            &self.inner
        }
    }
}


fn main() {

    nwg::init().expect("Failed to init Native Windows GUI");  // 初始化 GUI 并处理错误
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");  // 设置默认字体
    let _ui = BasicApp::build_ui(Default::default()).expect("Failed to build UI");  // 构建 UI
    nwg::dispatch_thread_events();  // 启动事件循环
    
}

