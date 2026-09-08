use super::rule::AssocRule;

pub fn get_default_rules() -> Vec<AssocRule> {
    if cfg!(target_os = "windows") {
        vec![
            AssocRule {
                mask: "*.rs".to_string(),
                open_cmd: "notepad %f".to_string(),
                view_cmd: None,
            },
            AssocRule {
                mask: "*.toml".to_string(),
                open_cmd: "notepad %f".to_string(),
                view_cmd: None,
            },
            AssocRule {
                mask: "*.md".to_string(),
                open_cmd: "notepad %f".to_string(),
                view_cmd: None,
            },
            AssocRule {
                mask: "*.{txt,json,yaml,yml,xml,ini,conf,cfg}".to_string(),
                open_cmd: "notepad %f".to_string(),
                view_cmd: None,
            },
            AssocRule {
                mask: "*.{sh,bat,cmd,ps1,py,pl,rb,js,ts}".to_string(),
                open_cmd: "notepad %f".to_string(),
                view_cmd: None,
            },
            AssocRule {
                mask: "*.{zip,tar,gz,bz2,xz,7z}".to_string(),
                open_cmd: "explorer %f".to_string(),
                view_cmd: None,
            },
            AssocRule {
                mask: "*.{jpg,jpeg,png,gif,bmp,svg,webp}".to_string(),
                open_cmd: "explorer %f".to_string(),
                view_cmd: None,
            },
            AssocRule {
                mask: "*.{mp3,wav,ogg,flac,m4a,mp4,mkv,avi,mov,wmv,webm}".to_string(),
                open_cmd: "explorer %f".to_string(),
                view_cmd: None,
            },
            AssocRule {
                mask: "*.{pdf,doc,docx,xls,xlsx,ppt,pptx}".to_string(),
                open_cmd: "explorer %f".to_string(),
                view_cmd: None,
            },
            AssocRule {
                mask: "*.{html,htm}".to_string(),
                open_cmd: "explorer %f".to_string(),
                view_cmd: None,
            },
        ]
    } else {
        vec![
            AssocRule {
                mask: "*.rs".to_string(),
                open_cmd: "nano %f".to_string(),
                view_cmd: Some("less %f".to_string()),
            },
            AssocRule {
                mask: "*.toml".to_string(),
                open_cmd: "nano %f".to_string(),
                view_cmd: Some("less %f".to_string()),
            },
            AssocRule {
                mask: "*.md".to_string(),
                open_cmd: "nano %f".to_string(),
                view_cmd: Some("less %f".to_string()),
            },
            AssocRule {
                mask: "*.{txt,json,yaml,yml,xml,ini,conf,cfg}".to_string(),
                open_cmd: "nano %f".to_string(),
                view_cmd: Some("less %f".to_string()),
            },
            AssocRule {
                mask: "*.{sh,bat,cmd,ps1,py,pl,rb,js,ts}".to_string(),
                open_cmd: "nano %f".to_string(),
                view_cmd: Some("less %f".to_string()),
            },
            AssocRule {
                mask: "*.{zip,tar,gz,bz2,xz,7z}".to_string(),
                open_cmd: "xdg-open %f".to_string(),
                view_cmd: None,
            },
            AssocRule {
                mask: "*.{jpg,jpeg,png,gif,bmp,svg,webp}".to_string(),
                open_cmd: "xdg-open %f".to_string(),
                view_cmd: None,
            },
            AssocRule {
                mask: "*.{mp3,wav,ogg,flac,m4a,mp4,mkv,avi,mov,wmv,webm}".to_string(),
                open_cmd: "xdg-open %f".to_string(),
                view_cmd: None,
            },
            AssocRule {
                mask: "*.{pdf,doc,docx,xls,xlsx,ppt,pptx}".to_string(),
                open_cmd: "xdg-open %f".to_string(),
                view_cmd: None,
            },
            AssocRule {
                mask: "*.{html,htm}".to_string(),
                open_cmd: "xdg-open %f".to_string(),
                view_cmd: None,
            },
        ]
    }
}
