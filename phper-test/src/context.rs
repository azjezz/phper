// Copyright (c) 2022 PHPER Framework Team
// PHPER is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2. You may obtain a copy of Mulan PSL v2 at:
//          http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use crate::utils;
use once_cell::sync::OnceCell;
use std::{
    env,
    fs::read_to_string,
    io::Write,
    ops::{Deref, DerefMut},
    path::Path,
    process::Command,
};
use tempfile::NamedTempFile;

pub struct Context {
    pub php_bin: String,
    pub ini_content: String,
}

impl Context {
    pub fn get_global() -> &'static Context {
        static CONTEXT: OnceCell<Context> = OnceCell::new();
        CONTEXT.get_or_init(|| {
            let mut ini_content = String::new();

            let php_config = env::var("PHP_CONFIG").unwrap_or_else(|_| "php-config".to_string());
            let php_bin = utils::execute_command(&[php_config.as_str(), "--php-binary"]);
            let ini_file = utils::execute_command(&[
                php_bin.as_str(),
                "-d",
                "display_errors=stderr",
                "-r",
                "echo php_ini_loaded_file();",
            ]);
            let ini_files = utils::execute_command(&[
                php_bin.as_str(),
                "-d",
                "display_errors=stderr",
                "-r",
                "echo php_ini_scanned_files();",
            ]);

            if !ini_file.is_empty() {
                ini_content.push_str(&read_to_string(ini_file).unwrap());
            }
            if !ini_files.is_empty() {
                for file in ini_files.split(',') {
                    let file = file.trim();
                    if !file.is_empty() {
                        ini_content.push_str(&read_to_string(file).unwrap());
                    }
                }
            }

            Context {
                php_bin,
                ini_content,
            }
        })
    }

    pub fn create_tmp_php_ini_file(&self, lib_path: impl AsRef<Path>) -> NamedTempFile {
        let mut out_ini_temp_file = NamedTempFile::new().unwrap();
        let out_ini_file = out_ini_temp_file.as_file_mut();

        out_ini_file.write_all(self.ini_content.as_bytes()).unwrap();
        out_ini_file
            .write_fmt(format_args!(
                "extension={}\n",
                lib_path.as_ref().to_str().unwrap()
            ))
            .unwrap();

        out_ini_temp_file
    }

    pub fn create_command_with_tmp_php_ini_args(
        &self, tmp_php_ini_file: &NamedTempFile, script: impl AsRef<Path>,
    ) -> ContextCommand {
        let mut cmd = Command::new(&self.php_bin);
        let args = vec![
            "-n".to_owned(),
            "-c".to_owned(),
            tmp_php_ini_file.path().to_str().unwrap().to_owned(),
            script.as_ref().to_str().unwrap().to_owned(),
        ];
        cmd.args(&args);
        ContextCommand { cmd, args }
    }

    #[cfg(feature = "fpm")]
    #[cfg_attr(docsrs, doc(cfg(feature = "fpm")))]
    pub fn find_php_fpm(&self) -> Option<String> {
        use std::ffi::OsStr;

        let php_bin = Path::new(&self.php_bin);
        php_bin.parent().and_then(Path::parent).and_then(|p| {
            php_bin
                .file_name()
                .and_then(OsStr::to_str)
                .and_then(|name| {
                    let mut p = p.to_path_buf();
                    p.push("sbin");
                    p.push(&format!(
                        "php-fpm{}",
                        if name.starts_with("php") {
                            name.chars().skip(3).collect::<String>()
                        } else {
                            "".to_owned()
                        }
                    ));
                    p.as_path().to_str().map(ToOwned::to_owned)
                })
        })
    }

    #[cfg(feature = "fpm")]
    #[cfg_attr(docsrs, doc(cfg(feature = "fpm")))]
    pub fn create_tmp_fpm_conf_file(&self) -> NamedTempFile {
        let mut tmp = NamedTempFile::new().unwrap();
        let file = tmp.as_file_mut();
        file.write_all(include_bytes!("../etc/php-fpm.conf"))
            .unwrap();
        tmp
    }
}

pub struct ContextCommand {
    cmd: Command,
    args: Vec<String>,
}

impl ContextCommand {
    pub fn get_args(&self) -> &[String] {
        &self.args
    }
}

impl Deref for ContextCommand {
    type Target = Command;

    fn deref(&self) -> &Self::Target {
        &self.cmd
    }
}

impl DerefMut for ContextCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cmd
    }
}
