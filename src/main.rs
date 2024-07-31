#![windows_subsystem = "windows"]

use std::{error::Error, path::PathBuf, vec};
use minigrep::Config;
use std::fs;
extern crate native_windows_gui as nwg;  // 将 `native_windows_gui` 库引入并重命名为 `nwg`
use nwg::NativeUi;
use clipboard_win::{Clipboard, formats,set_clipboard};
use std::path::Path;

use zip::read::ZipArchive;
use std::io::{self, Read, Seek,Cursor};
use encoding_rs::GBK;
use flate2::read::GzDecoder;
use std::str::from_utf8;
 
struct MatchResult {
    matched_text: String,
    file_name: String,
    line_number: String,
}


#[derive(Default)]
pub struct  FeatureLayout {
    id: usize,
    list_box: nwg::ListBox<String>,
    input_text: nwg::TextInput,
    add_button: nwg::Button,
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
                self.list_box.push(r"(?<!\d)(1\d{10})(?!\d)".to_string());
                self.list_box.push(r"[a-zA-Z0-9\*]+\@[a-zA-Z0-9]+\.[a-zA-Z]+".to_string());
                self.list_box.push(r"(?<!\d)(\d{17}[Xx]|\d{18})(?!\d)".to_string());
                self.list_box.push("((P|p)ass(P|p)ort((N|n)o(s|S)?)?(\\s)?\"?(\\s)?\\:(\\s)?(\\[)?\"?[a-zA-Z0-9]+\"?[,;]+)".to_string()); //|((P|p)ass(P|p)ort((N|n)o)?\\:(\\t)?[a-zA-Z0-9]+)
            },
            1 => {  
                self.list_box.push(r"passwd|password|PASSWORD|PASSWD|PassWd|PassWD|PassWord".to_string());
                self.list_box.push(r"appkey|AppKey|appKey|appKEY|AppKEY|APPKEY".to_string());
                self.list_box.push(r"skey|SKey|SKEY|sKey|sKEY".to_string());
                self.list_box.push(r"access_token".to_string());
                self.list_box.push("(T|t)oken\\\"\\:\t".to_string());
                self.list_box.push("(S|s)ecret\\\"\\:\t".to_string());
            },
            _ => {}  // 其他列表框可以在这里添加其他默认元素
        }
    }
}


#[derive(Default)]  // 自动为结构体实现 Default trait
pub struct BasicApp {  // 定义一个名为 BasicApp 的公共结构体
    window: nwg::Window,  // 窗口组件
    layout: nwg::GridLayout,  // 网格布局管理器

    features: Vec<FeatureLayout>,
    path_input_text: nwg::TextInput,
    filedialog: nwg::FileDialog,
    browse_button: nwg::Button,
    check_button: nwg::Button,
    clear_button: nwg::Button,
    list_view: nwg::ListView,
    tis: nwg::Label,
    dyn_tis: nwg::Label,

}

impl BasicApp {
    fn handle_button_click(&self, button_handle: &nwg::ControlHandle) {
        for feature in &self.features {
            if button_handle == &feature.add_button.handle {
                self.add_item(feature.id);
            } else if button_handle == &feature.remove_button.handle {
                self.remove_item(feature.id);
            } else if button_handle == &feature.clear_button.handle {
                self.clear_item(feature.id);
            }
        }
        
    }

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
                "gz" => {
                    let file = fs::File::open(&path)?;
                    all_results.extend(self.process_gz_file(&regex_list, file, &path, base_dir)?);
                },
                "tar" => {
                    let file = fs::File::open(&path)?;
                    all_results.extend(self.process_tar_bytes(&regex_list, file, &path, base_dir)?);
                },
                _ => {
                    let contents = match fs::read_to_string(&path) {
                        Ok(c) => c,
                        Err(_) => {
                            self.dyn_tis.set_text(format!("文件 {} 不是文本文件，跳过检索", &path.to_string_lossy()).as_str());
                            return Ok(all_results);
                        }  // 如果失败，返回错误
                    };
                    all_results.extend(self.search_in_file_contents(&regex_list, &contents, &path, &self.strip_base_dir(base_dir, &path)));
                }
            }
        }
        Ok(all_results)
    }
    
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
                } else {
                    let mut contents = String::new();
                    file.read_to_string(&mut contents)?;
                    all_results.extend(self.search_in_file_contents(&regex_list, &contents, Path::new(&relative_path), &relative_path));
                }
            }
        }
    
        Ok(all_results)
    }
    
    

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
        if gz_path.extension().and_then(std::ffi::OsStr::to_str) == Some("tar.gz") {
            let cursor = Cursor::new(&decompressed_data);
            return self.process_tar_bytes(regex_list, cursor, gz_path, base_dir);
        }
        let cursor = Cursor::new(&decompressed_data);
        // 进一步检查解压后的文件类型
        let mut archive = ZipArchive::new(cursor.clone());
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
    
        let contents = match String::from_utf8(decompressed_data) {
            Ok(c) => c,
            Err(_) => {
                self.dyn_tis.set_text(format!("文件 {} 不是文本文件，跳过检索", gz_path.to_string_lossy()).as_str());
                return Ok(all_results);
            }
        };
    
        let relative_path = self.strip_base_dir(base_dir, gz_path);
        all_results.extend(self.search_in_file_contents(&regex_list, &contents, gz_path, &relative_path));
    
        Ok(all_results)
    }
    
    
    

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
    
            let mut relative_path = format!("{}/{}", self.strip_base_dir(base_dir, tar_path), file_name);
    
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
            } else {
                let contents_str = from_utf8(&contents).unwrap_or("");
                all_results.extend(self.search_in_file_contents(&regex_list, contents_str, Path::new(&relative_path), &relative_path));
            }
    
            let remainder = 512 - (size % 512);
            if remainder < 512 {
                let mut skip = vec![0; remainder];
                reader.read_exact(&mut skip)?;
            }
        }
    
        Ok(all_results)
    }
    
    fn process_war_file<R: Read + Seek>(&self, regex_list: &[String], reader: R, war_path: &Path, base_dir: &Path) -> Result<Vec<MatchResult>, Box<dyn Error>> {
        // WAR 文件本质上是 ZIP 文件，所以我们可以调用 process_zip_file
        self.process_zip_file(regex_list, reader, war_path, base_dir)
    }

    fn search_in_file_contents(&self, regex_list: &[String], contents: &str, path: &Path, file_name: &str) -> Vec<MatchResult> {
        let mut results = Vec::new();
        for query in regex_list {
            let config = Config {
                query: query.clone(),
                contents: contents.to_string(),
                ignore_case: false,
            };
            if let Ok(matches) = minigrep::run(config) {
                for (line_number, matched_text) in matches {
                    let display_file_name = if path.is_file() {
                        file_name.to_string()  // 直接使用文件名，不需要额外的路径信息
                    } else {
                        format!("{}", file_name)
                    };
                    results.push(MatchResult {
                        file_name: display_file_name,
                        line_number,
                        matched_text,
                    });
                }
            }
        }
        results
    }
    

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

    fn begin_check(&self) {
        let directory = self.path_input_text.text();
        if directory.len() == 0 {
            self.path_input_text.set_text("请输入日志目录");
            return
        }
        let regex_list: Vec<String> = self.get_check_regex_list();
        let all_results = self.get_all_file(regex_list, directory);
        match all_results {
            Ok(all_res) => {
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
                        text: Some(result.matched_text),
                        index: Some(list_view_num as i32),
                        image: None,
                    });
                    self.list_view.insert_item(nwg::InsertListViewItem {
                        column_index: 2,
                        text: Some(format!("{} 第 {} 行", result.file_name, result.line_number)),
                        index: Some(list_view_num as i32),
                        image: None,
                    });      
                }
            },
            _ => { self.dyn_tis.set_text("该目录或文件中有文件内容为非文本内容，筛查失败，请检查后再试") }
        }
    }

    fn clear_list_view(&self) {
        self.list_view.clear();
    }

    fn add_item(&self, feature_id: usize) {
        let feature = &self.features[feature_id];
        let text = feature.input_text.text();
        if !text.is_empty() {
            feature.list_box.push(text);
            feature.input_text.set_text("");
        }
    }

    fn remove_item(&self, feature_id: usize) {
        let feature = &self.features[feature_id];
        if let Some(selected) = feature.list_box.selection() {
            feature.list_box.remove(selected);
        }
    }

    fn clear_item(&self, feature_id: usize) {
        let feature = &self.features[feature_id];
        feature.list_box.clear();
    }

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
                self.path_input_text.set_text(c.as_str());
            }
        }
    }
    
    fn tmp(&self,handle: &nwg::ControlHandle) {
        if let Some(index) = self.list_view.selected_item() {
            if let Some(item1) = self.list_view.item(index,1,100) {
                if let Some(item2) = self.list_view.item(index,2,100) {
                    // let text = item1.text + " " + item2.text.as_str();
                    if let Err(e) = set_clipboard(formats::Unicode, item1.text){

                    };
                }
            }
        }
        
    }
    fn path_copy(&self,handle: &nwg::ControlHandle) {
        if let Some(index) = self.list_view.selected_item() {
            if let Some(item1) = self.list_view.item(index,1,100) {
                if let Some(item2) = self.list_view.item(index,2,100) {
                    // let text = item1.text + " " + item2.text.as_str();
                    if let Err(e) = set_clipboard(formats::Unicode, item2.text){

                    };
                }
            }
        }
        
    }
    fn strip_base_dir(&self, base_dir: &Path, full_path: &Path) -> String {
        full_path.strip_prefix(base_dir)
            .unwrap_or(full_path)
            .to_string_lossy()
            .to_string()
    }
}
    

mod basic_app_ui {  // 定义一个模块，用于用户界面的管理
    use super::*; 
    use std::process::exit;
    // 引入上级作用域中的所有项
    use std::rc::Rc;  // 使用 Rc 用于引用计数的智能指针
    use std::cell::RefCell;  // 使用 RefCell 提供内部可变性
    use std::ops::Deref;  // 引入 Deref trait 用于自定义解引用行为
    use nwg::CheckBoxState;

    pub struct BasicAppUi {  // 定义 UI 管理结构体
        inner: Rc<BasicApp>,  // 使用 Rc 封装 BasicApp，允许多处共享所有权
        default_handler: RefCell<Option<nwg::EventHandler>>  // 事件处理器，用 RefCell 提供内部可变性
    }

    impl nwg::NativeUi<BasicAppUi> for BasicApp {  // 实现 NativeUi trait 用于构建 UI
        fn build_ui(mut data: BasicApp) -> Result<BasicAppUi, nwg::NwgError> {  // 构建 UI 并返回 BasicAppUi 或错误
            use nwg::Event as E;  // 引入 Event 枚举简化名称
            
            // 创建窗口
            nwg::Window::builder()
                .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)  // 窗口属性
                .size((1100, 1000))  // 窗口大小
                .position((150, 80))  // 窗口位置
                .accept_files(true)
                //.center(true)
                .title("日志敏感信息查询 by nekous v1.2")  // 窗口标题
                //.maximized(true)
                .build(&mut data.window)?;  // 构建窗口并处理错误

            // 添加输入框和按钮
            nwg::TextInput::builder()
                .parent(&data.window)
                .text("此处粘贴日志目录")  // 初始文本为空
                .build(&mut data.path_input_text)?;

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

            nwg::ListView::builder()
                .parent(&data.window)
                .item_count(4)
                .list_style(nwg::ListViewStyle::Detailed)  // 设置为报表样式，支持列标题
                .build(&mut data.list_view)
                .expect("Failed to create list view");

            nwg::Label::builder()
                .parent(&data.window)
                .text("注意:本工具不能完全代替日志筛查,仅能用来筛查敏感信息\n日志问题还包括行为记录不足,并可能存在遗漏,请手动排查\n划选多个文件为选择该目录，等同选取该目录下所有文件\n右键点击id即可复制匹配值，左键点击复制文件名路径\n新增压缩包扫描 7z除外")
                .build(&mut data.tis)
                .expect("文字展示出错");

            nwg::Label::builder()
                .parent(&data.window)
                .text("这里是提示框\n1. 移除通用ip匹配，请手动添加伪造ip \n2. 修复gz扫描，优化文件名显示")
                .build(&mut data.dyn_tis)
                .expect("动态文字展示出错");

            nwg::FileDialog::builder()
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
            let ena = data.list_view.enabled();



            let layout_list = vec!["身份信息(手机号，邮箱，证件号)","密钥token"].into_iter();
            data.features = layout_list.enumerate().map(|(index,_name)| FeatureLayout {
                id:index ,
                list_box: Default::default(),
                input_text: Default::default(),
                add_button: Default::default(),
                remove_button: Default::default(),
                clear_button: Default::default(),
                able_checkbox: Default::default(),
                // regex_checkbox: Default::default(),
                divider: Default::default(),
            }).collect::<Vec<_>>();
            let layout_list = vec!["身份信息(手机号，邮箱，证件号)","密钥token"].into_iter();
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

            


            // Event handling
            let ui = BasicAppUi {
                inner: Rc::new(data),
                default_handler: Default::default(),
            };

            // 事件绑定
            let evt_ui = Rc::downgrade(&ui.inner);
            let handle_events = move |evt, _evt_data, handle| {
                if let Some(ui) = evt_ui.upgrade() {
                    match evt {
                        E::OnButtonClick => {
                            if &handle == &ui.browse_button.handle {
                                ui.open_file_dialog(&handle);  // 调用打开文件对话框的方法
                            } else if &handle == &ui.check_button{
                                ui.begin_check();
                            } else if &handle == &ui.clear_button{
                                ui.clear_list_view();
                            }else {
                                ui.handle_button_click(&handle);  // 处理其他按钮点击事件
                            }
                        },
                        E::OnWindowClose => exit(1),
                        E::OnListViewRightClick => ui.tmp(&handle),
                        E::OnListViewClick  => ui.path_copy(&handle),
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
                    .child_item(nwg::GridLayoutItem::new(&x.remove_button, col_num + 1, row_num + 3, 1, 1))
                    .child_item(nwg::GridLayoutItem::new(&x.clear_button, col_num + 1, row_num + 4, 1, 1))
                    .child_item(nwg::GridLayoutItem::new(&x.divider, col_num, row_num + 6, 2, 1));
                row_num += 7;
                col_index += 1;
                x.initialize_defaults();
            
            }
            tmp = tmp.child_item(nwg::GridLayoutItem::new(&ui.dyn_tis, col_num, row_num , 2, 2))
                .child_item(nwg::GridLayoutItem::new(&ui.path_input_text, col_num, row_num + 2, 1, 1))
                .child_item(nwg::GridLayoutItem::new(&ui.browse_button, col_num +1 , row_num + 2, 1, 1))
                .child_item(nwg::GridLayoutItem::new(&ui.check_button, col_num , row_num + 3, 1, 1))
                .child_item(nwg::GridLayoutItem::new(&ui.clear_button, col_num + 1 , row_num + 3, 1, 1))
                .child_item(nwg::GridLayoutItem::new(&ui.tis, col_num, row_num + 5, 2, 2))
                .child_item(nwg::GridLayoutItem::new(&ui.list_view, col_num + 2 , 0, 2, 21));

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
