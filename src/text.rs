// src/text.rs

pub const TITLE: &str = "minigrep by nekous v1.5";
pub const ABOUT_TEXT: &str = "注意:
1. 本工具不能完全代替日志筛查,仅能用来筛查敏感信息
2. 日志问题还包括行为记录不足,并可能存在遗漏,请手动排查
3. 划选多个文件为选择该目录，等同选取该目录下所有文件
4. 右键点击id即可复制匹配值，左键点击展示匹配行与路径
5. 新增压缩包扫描 7z除外";
pub const UPDATE_LOG: &str = "
version 1.5
1. 增加了发布包扫描及其规则库
2. 左键点击修改为展示所在行及路径，右键点击保持不变
3. 增加了所在行及上下行展示框（默认发布包模式生效，可手动切换），所在行默认显示在中间

version 1.4\n1. 增加了直接拖拽文件功能\n2. 增加了配置文件用以保存个人规则库\n\nversion 1.3\n1. 增加了对gbk格式文件的支持\n2. 调整了默认规则候选框\n3. 优化了UI界面";


// text.rs
pub const LOG_RULES: &[(&str, &[&str])] = &[
    ("日志规则库", &[
        r"(?<!\d)(1\d{10})(?!\d)",  // 手机号
        r"[a-zA-Z0-9\*]+\@[a-zA-Z0-9]+\.[a-zA-Z]+",  // 邮箱
        r"(?<!\d)(\d{17}[Xx]|\d{18})(?!\d)",  // 身份证号
        "((P|p)ass(P|p)ort((N|n)o(s|S)?)?(\\s)?\"?(\\s)?\\:(\\s)?(\\[)?\"?[a-zA-Z0-9]+\"?[,;]+)",  // 护照号码
    ]),
    ("关键字匹配", &[
        r"(P|p)(A|a)(S|s)(S|s)(W|w)((O|o)(R|r))?(D|d)",  // 更宽泛的密码匹配
        r"(A|a)(E|e)(S|s)_?(K|k)(E|e)(Y|y)",  // AES key 匹配
        r"Pwd|PWD|pwd",
        r"appkey|AppKey|appKey|appKEY|AppKEY|APPKEY",
        r"skey|SKey|SKEY|sKey|sKEY",
        r"access_token",
        "(T|t)oken\\\"\\:\t",
        "(S|s)ecret\\\"\\:\t",
        "(C|c)ertificate",
        "(I|i)(D|d)_?(C|c)ard",
    ]),
];

pub const PACKAGE_RULES: &[(&str, &[&str])] = &[
    ("发布包规则库", &[
        r"(J|j)(W|w)(T|t)\\.(A|a)(L|l)(G|g)(O|o)(R|r)(I|i)(T|t)(H|h)(M|m)",  // JWT算法
        r"(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)",  // SECRET 匹配
        r"(P|p)(A|a)(S|s)(S|s)(W|w)((O|o)(R|r))?(D|d)",  // 更宽泛的密码匹配
        r"(A|a)(E|e)(S|s)_?(K|k)(E|e)(Y|y)",  // AES key 匹配
    ]),
];
