use ratatui::style::Color;

pub struct FileIcon {
    pub icon: &'static str,
    pub color: Color,
}

pub fn icon_for_extension(ext: &str) -> FileIcon {
    match ext.to_lowercase().as_str() {
        "rs" => FileIcon { icon: "\u{e7a8}", color: Color::Rgb(222, 165, 132) },    //
        "py" | "pyw" => FileIcon { icon: "\u{e73c}", color: Color::Rgb(55, 118, 171) },  //
        "js" | "mjs" | "cjs" => FileIcon { icon: "\u{e781}", color: Color::Rgb(241, 224, 90) }, //
        "ts" | "mts" => FileIcon { icon: "\u{e628}", color: Color::Rgb(49, 120, 198) },  //
        "tsx" => FileIcon { icon: "\u{e7ba}", color: Color::Rgb(49, 120, 198) },    //
        "jsx" => FileIcon { icon: "\u{e7ba}", color: Color::Rgb(241, 224, 90) },    //
        "go" => FileIcon { icon: "\u{e627}", color: Color::Rgb(0, 173, 216) },      //
        "rb" => FileIcon { icon: "\u{e739}", color: Color::Rgb(204, 52, 45) },      //
        "java" => FileIcon { icon: "\u{e738}", color: Color::Rgb(204, 62, 68) },    //
        "c" => FileIcon { icon: "\u{e61e}", color: Color::Rgb(85, 135, 195) },      //
        "h" => FileIcon { icon: "\u{e61e}", color: Color::Rgb(120, 160, 210) },     //
        "cpp" | "cxx" | "cc" => FileIcon { icon: "\u{e61d}", color: Color::Rgb(85, 135, 195) }, //
        "hpp" | "hxx" => FileIcon { icon: "\u{e61d}", color: Color::Rgb(120, 160, 210) }, //
        "cs" => FileIcon { icon: "\u{f81a}", color: Color::Rgb(96, 129, 32) },      //
        "swift" => FileIcon { icon: "\u{e755}", color: Color::Rgb(255, 106, 48) },  //
        "kt" | "kts" => FileIcon { icon: "\u{e634}", color: Color::Rgb(139, 100, 209) }, //
        "php" => FileIcon { icon: "\u{e73d}", color: Color::Rgb(119, 123, 180) },   //
        "html" | "htm" => FileIcon { icon: "\u{e736}", color: Color::Rgb(228, 77, 38) }, //
        "css" => FileIcon { icon: "\u{e749}", color: Color::Rgb(66, 165, 245) },    //
        "scss" | "sass" => FileIcon { icon: "\u{e603}", color: Color::Rgb(204, 102, 153) }, //
        "json" => FileIcon { icon: "\u{e60b}", color: Color::Rgb(203, 203, 65) },   //
        "yaml" | "yml" => FileIcon { icon: "\u{e6a8}", color: Color::Rgb(203, 171, 110) }, //
        "toml" => FileIcon { icon: "\u{e6b2}", color: Color::Rgb(156, 156, 156) },  //
        "xml" => FileIcon { icon: "\u{e619}", color: Color::Rgb(227, 127, 43) },    //
        "md" | "markdown" => FileIcon { icon: "\u{e73e}", color: Color::Rgb(66, 165, 245) }, //
        "sql" => FileIcon { icon: "\u{e706}", color: Color::Rgb(218, 179, 77) },    //
        "sh" | "bash" | "zsh" => FileIcon { icon: "\u{e795}", color: Color::Rgb(137, 224, 81) }, //
        "lua" => FileIcon { icon: "\u{e620}", color: Color::Rgb(81, 160, 207) },    //
        "vim" => FileIcon { icon: "\u{e62b}", color: Color::Rgb(1, 152, 51) },      //
        "zig" => FileIcon { icon: "\u{e6a9}", color: Color::Rgb(236, 145, 18) },    //
        "ex" | "exs" => FileIcon { icon: "\u{e62d}", color: Color::Rgb(111, 68, 149) }, //
        "erl" | "hrl" => FileIcon { icon: "\u{e7b1}", color: Color::Rgb(161, 0, 24) }, //
        "hs" => FileIcon { icon: "\u{e777}", color: Color::Rgb(94, 80, 134) },      //
        "r" => FileIcon { icon: "\u{f25d}", color: Color::Rgb(39, 108, 181) },      //
        "dart" => FileIcon { icon: "\u{e798}", color: Color::Rgb(0, 180, 216) },    //
        "vue" => FileIcon { icon: "\u{e6a0}", color: Color::Rgb(65, 184, 131) },    //
        "svelte" => FileIcon { icon: "\u{e697}", color: Color::Rgb(255, 62, 0) },   //
        "docker" | "dockerfile" => FileIcon { icon: "\u{e7b0}", color: Color::Rgb(56, 150, 218) }, //
        "makefile" | "mk" => FileIcon { icon: "\u{e779}", color: Color::Rgb(111, 111, 111) }, //
        "git" | "gitignore" | "gitmodules" => FileIcon { icon: "\u{e702}", color: Color::Rgb(240, 80, 50) }, //
        "lock" => FileIcon { icon: "\u{f023}", color: Color::Rgb(150, 150, 150) },   //
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "ico" | "webp" => {
            FileIcon { icon: "\u{f1c5}", color: Color::Rgb(165, 130, 210) }          //
        }
        "mp3" | "wav" | "ogg" | "flac" => {
            FileIcon { icon: "\u{f1c7}", color: Color::Rgb(210, 130, 165) }          //
        }
        "mp4" | "avi" | "mov" | "mkv" | "webm" => {
            FileIcon { icon: "\u{f1c8}", color: Color::Rgb(255, 180, 100) }          //
        }
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => {
            FileIcon { icon: "\u{f1c6}", color: Color::Rgb(175, 160, 130) }          //
        }
        "pdf" => FileIcon { icon: "\u{f1c1}", color: Color::Rgb(220, 50, 47) },      //
        "txt" => FileIcon { icon: "\u{f15c}", color: Color::Rgb(170, 170, 170) },    //
        "log" => FileIcon { icon: "\u{f18d}", color: Color::Rgb(140, 140, 140) },    //
        "env" => FileIcon { icon: "\u{f462}", color: Color::Rgb(250, 210, 50) },     //
        "tf" => FileIcon { icon: "\u{e69a}", color: Color::Rgb(81, 66, 224) },       //
        "proto" => FileIcon { icon: "\u{e6b1}", color: Color::Rgb(140, 140, 200) },  //
        _ => FileIcon { icon: "\u{f15b}", color: Color::Rgb(150, 150, 150) },         //
    }
}

pub fn icon_for_directory(expanded: bool) -> FileIcon {
    if expanded {
        FileIcon {
            icon: "\u{f115}",  //
            color: Color::Rgb(220, 200, 120),
        }
    } else {
        FileIcon {
            icon: "\u{f114}",  //
            color: Color::Rgb(220, 200, 120),
        }
    }
}

pub fn icon_for_directory_special(name: &str, expanded: bool) -> Option<FileIcon> {
    match name.to_lowercase().as_str() {
        ".git" => Some(FileIcon { icon: "\u{e702}", color: Color::Rgb(240, 80, 50) }),
        "src" | "lib" => Some(FileIcon {
            icon: if expanded { "\u{f115}" } else { "\u{f114}" },
            color: Color::Rgb(120, 180, 255),
        }),
        "test" | "tests" | "spec" | "specs" => Some(FileIcon {
            icon: if expanded { "\u{f115}" } else { "\u{f114}" },
            color: Color::Rgb(120, 220, 120),
        }),
        "node_modules" => Some(FileIcon { icon: "\u{e718}", color: Color::Rgb(120, 160, 60) }),
        "target" | "build" | "dist" | "out" => Some(FileIcon {
            icon: if expanded { "\u{f115}" } else { "\u{f114}" },
            color: Color::Rgb(180, 150, 100),
        }),
        _ => None,
    }
}
