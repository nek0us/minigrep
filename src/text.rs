// src/text.rs

pub const TITLE: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/VERSION"));



pub const ABOUT_TEXT: &str = "注意:
1. 本工具不能完全代替日志筛查,仅能用来筛查敏感信息
2. 日志问题还包括行为记录不足,并可能存在遗漏,请手动排查
3. 划选多个文件为选择该目录，等同选取该目录下所有文件
4. 右键点击id即可复制匹配值，左键点击展示匹配行与路径
5. 新增压缩包扫描 7z除外

6. 发布包扫描需要自行添加好java环境变量
7. 发布包扫描会反编译，所以速度较慢
";
pub const UPDATE_LOG: &str = "
version 1.7.1
1. html报告优化

version 1.7.0
1. 添加扫描结果导出功能
2. 移除【清空列表】按钮

version 1.61
1. 修复 1.6 release 版本在部分情况下扫描结果为空的问题

version 1.6
1. 添加了发布包代码反编译扫描，需要java环境
2. 添加了发布包匹配规则库，默认取消发布包关键词匹配
3. 优化了匹配内容的高亮，快速定位，多匹配值提示
4. 优化了匹配性能问题

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
        r"(A|a)(P|p)(P|p)_?(K|k)(E|e)(Y|y)",
        r"(S|s)_?(K|k)(E|e)(Y|y)",
        r"(A|a)ccess_?(T|t)oken",
        "(T|t)oken\\\"\\:\t",
        "(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)\\\"\\:\t",
        "(C|c)ertificate",
        "(I|i)(D|d)_?(C|c)ard",
    ]),
];

pub const PACKAGE_RULES: &[(&str, &[&str])] = &[
    ("发布包规则匹配", &[
        r#"((P|p)((A|a)(S|s)(S|s))?(W|w)((O|o)(R|r))?(D|d)|(K|k)(E|e)(Y|y)|(E|e)(N|n)(C|c)(R|r)(Y|y)(P|p)(T|t)|(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)|(A|a)(U|u)(T|t)(H|h)((O|o)(R|r)(I|i)(Z|z)(A|a)(T|t)(I|i)(O|o)(N|n))?)\s?[\"\']?(=|:)+\s?[\"\']?[a-zA-Z0-9\@\.]+[\"\']?"#,  // 因地制宜的密钥匹配,在class中会被替换，强制搜索引号包裹的
        r#"((P|p)((A|a)(S|s)(S|s))?(W|w)((O|o)(R|r))?(D|d)|(K|k)(E|e)(Y|y)|(E|e)(N|n)(C|c)(R|r)(Y|y)(P|p)(T|t)|(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)|(A|a)(U|u)(T|t)(H|h)((O|o)(R|r)(I|i)(Z|z)(A|a)(T|t)(I|i)(O|o)(N|n))?)[\"\']?\s?value(=|:)+[\"\']?[a-zA-Z0-9\@\.]+[\"\']?"#,
        r#"((P|p)((A|a)(S|s)(S|s))?(W|w)((O|o)(R|r))?(D|d)|(K|k)(E|e)(Y|y)|(E|e)(N|n)(C|c)(R|r)(Y|y)(P|p)(T|t)|(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)|(A|a)(U|u)(T|t)(H|h)((O|o)(R|r)(I|i)(Z|z)(A|a)(T|t)(I|i)(O|o)(N|n))?)[\"\']?\>)+\s?[a-zA-Z0-9\@\.]+\<[\"\']?"#,
        r#"(S|s)(E|e)(T|t)([a-zA-Z0-9]+)?((P|p)((A|a)(S|s)(S|s))?(W|w)((O|o)(R|r))?(D|d)|(K|k)(E|e)(Y|y)|(E|e)(N|n)(C|c)(R|r)(Y|y)(P|p)(T|t)|(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)|(A|a)(U|u)(T|t)(H|h)((O|o)(R|r)(I|i)(Z|z)(A|a)(T|t)(I|i)(O|o)(N|n))?)\(\s?[\"\']+[a-zA-Z0-9\@\.]+[\"\']+\s?\)"#,
        r#"[\"\']+[a-zA-Z0-9\@\.]+[\"\']+\s?\,\s?((P|p)((A|a)(S|s)(S|s))?(W|w)((O|o)(R|r))?(D|d)|(E|e)(N|n)(C|c)(R|r)(Y|y)(P|p)(T|t)|(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)|(A|a)(U|u)(T|t)(H|h)((O|o)(R|r)(I|i)(Z|z)(A|a)(T|t)(I|i)(O|o)(N|n))?)+"#,
        r#"((P|p)((A|a)(S|s)(S|s))?(W|w)((O|o)(R|r))?(D|d)|(E|e)(N|n)(C|c)(R|r)(Y|y)(P|p)(T|t)|(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)|(A|a)(U|u)(T|t)(H|h)((O|o)(R|r)(I|i)(Z|z)(A|a)(T|t)(I|i)(O|o)(N|n))?)+\s?\,\s?[\"\']+[a-zA-Z0-9\@\.]+[\"\']+"#,
    ]),
    ("发布包关键字匹配", &[
        r"(J|j)(W|w)(T|t)\\.(A|a)(L|l)(G|g)(O|o)(R|r)(I|i)(T|t)(H|h)(M|m)",  // JWT算法
        r"(S|s)(E|e)(C|c)(R|r)(E|e)(T|t)",  // SECRET 匹配
        r"(P|p)(A|a)(S|s)(S|s)(W|w)((O|o)(R|r))?(D|d)",  // 更宽泛的密码匹配
        r"(A|a)(E|e)(S|s)_?(K|k)(E|e)(Y|y)",  // AES key 匹配
    ]),
];

pub const HTML_HEAD: &str = r#"
<html>
    <head>
        <style>
            body {
                font-family: Arial, sans-serif;
                display: flex;
                background-color: #f8f9fa;
                color: #333;
            }
            #sidebar {
                width: 280px;
                height: 100vh;
                position: fixed;
                left: 0;
                top: 0;
                overflow-y: auto;
                padding: 20px;
                background-color: #343a40;
                border-right: 1px solid #ccc;
                color: #ffffff;
            }
            #sidebar h2 {
                color: #ffcc00;
            }
            #content {
                margin-left: 300px;
                padding: 20px;
                width: calc(100% - 300px);
                background-color: #ffffff;
                border-left: 1px solid #dee2e6;
            }
            .regex-section {
                background-color: #f1f1f1;
                padding: 15px;
                margin-bottom: 20px;
                border: 1px solid #ccc;
                border-radius: 5px;
            }
            .highlight {
                background-color: #ffcc00;
                font-weight: bold;
                color: #000000;
            }
            .line-content {
                font-family: monospace;
                white-space: pre-wrap;
                margin: 0;
                color: #444;
            }
            .file-section {
                margin-bottom: 20px;
                padding: 15px;
                border: 1px solid #ced4da;
                border-radius: 5px;
                background-color: #e9ecef;
            }
            .match {
                margin: 10px 0;
                background-color: #f8f9fa;
                border-radius: 5px;
                padding: 10px;
            }
            .code-container {
                display: flex;
                align-items: center;
                cursor: pointer;
                margin: 5px 0;
                padding: 8px;
                background-color: #f1f3f5;
                border-radius: 5px;
                transition: background-color 0.3s ease;
            }
            .code-container:hover {
                background-color: #e2e6ea;
            }
            .code-left, .code-right {
                font-family: monospace;
                white-space: nowrap;
                overflow: hidden;
                text-overflow: ellipsis;
            }
            .code-left {
                text-align: right;
                padding-right: 5px;
                color: #666;
                flex: 1;
            }
            .code-right {
                text-align: left;
                padding-left: 5px;
                color: #666;
                flex: 2;
            }
            .full-line {
                display: none;
                margin-top: 10px;
                font-family: monospace;
                background-color: #f8f9fa;
                padding: 15px;
                border: 1px solid #ced4da;
                border-radius: 5px;
            }
            .full-line.show {
                display: block;
            }
            a {
                text-decoration: none;
                color: #ffc107;
            }
            a:hover {
                text-decoration: underline;
                color: #ffdd57;
            }
        </style>
        <script>
            function toggleFullLine(id) {
                const fullLine = document.getElementById(id);
                const codeContainer = document.querySelector(`[data-id="${id}"]`);
                if (fullLine.classList.contains('show')) {
                    fullLine.classList.remove('show');
                    codeContainer.style.display = 'flex'; // 恢复原来的部分上下文显示
                } else {
                    fullLine.classList.add('show');
                    codeContainer.style.display = 'none'; // 隐藏部分上下文显示
                }
            }
            function scrollToMatch(id) {
                const element = document.getElementById(id);
                if (element) {
                    element.scrollIntoView({ behavior: 'smooth', block: 'start' });
                }
            }
        </script>
    </head>
    <body>
        <div id="sidebar">
            <h2>目录</h2>
"#;

pub const HTML_FOOTER: &str = r#"
        </div>
    </body>
</html>
"#;