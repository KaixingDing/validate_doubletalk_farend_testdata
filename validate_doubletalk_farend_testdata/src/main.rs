use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 检查指定目录下是否至少包含一个 .pcm 文件
fn check_pcm_files(directory_path: &Path) -> bool {
    if !directory_path.is_dir() {
        return false;
    }

    if let Ok(entries) = fs::read_dir(directory_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.to_string_lossy().to_lowercase() == "pcm" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// 执行目录结构质检的主函数
fn validate_directory_structure(root_dir: &Path) -> Vec<String> {
    let mut errors = Vec::new();

    if !root_dir.is_dir() {
        errors.push(format!("根目录错误: '{}' 不是一个有效的目录或不存在。", root_dir.display()));
        return errors;
    }

    // 遍历第二层目录（如日期）
    if let Ok(level_2_entries) = fs::read_dir(root_dir) {
        for level_2_entry in level_2_entries.flatten() {
            let level_2_path = level_2_entry.path();
            if !level_2_path.is_dir() {
                continue;
            }

            // 遍历第三层目录（如“单讲_动态”）
            if let Ok(level_3_entries) = fs::read_dir(&level_2_path) {

                // println!("正在质检目录: {}", level_2_path.display());

                for level_3_entry in level_3_entries.flatten() {
                    let level_3_path = level_3_entry.path();
                    let level_3_name = level_3_path.file_name().unwrap_or_default().to_string_lossy();

                    if !level_3_path.is_dir() {
                        continue;
                    }

                    println!("正在质检目录: {}", level_3_path.display());

                    let is_single = level_3_name.contains("单讲");
                    let is_double = level_3_name.contains("双讲");
                    let mut talk_type = None;

                    // 检查目录命名规则
                    if !is_single && !is_double {
                        errors.push(format!("层级3命名错误: 目录 '{}' 必须包含 '单讲' 或 '双讲'。", level_3_path.display()));
                        continue;
                    }

                    if is_single {
                        talk_type = Some("单讲");
                        let is_dynamic = level_3_name.contains("动态");
                        let is_static = level_3_name.contains("静态");

                        if !is_dynamic && !is_static {
                            errors.push(format!("层级3命名错误 ('单讲'): 目录 '{}' 必须包含 '动态' 或 '静态'。", level_3_path.display()));
                        }

                        // 单讲：第四层为 PCM 文件
                        if !check_pcm_files(&level_3_path) {
                            errors.push(format!("层级4内容错误 ('单讲'): 目录 '{}' 下必须包含至少一个 .pcm 文件。", level_3_path.display()));
                        }

                    } else if is_double {
                        talk_type = Some("双讲");

                        // 双讲：进入第四层（人名目录）
                        if let Ok(level_4_entries) = fs::read_dir(&level_3_path) {
                            for level_4_entry in level_4_entries.flatten() {
                                let level_4_path = level_4_entry.path();
                                if !level_4_path.is_dir() {
                                    continue;
                                }

                                // 第五层：APK 或 整轨 目录
                                if let Ok(level_5_entries) = fs::read_dir(&level_4_path) {
                                    for level_5_entry in level_5_entries.flatten() {
                                        let level_5_path = level_5_entry.path();
                                        let level_5_name = level_5_path.file_name().unwrap_or_default().to_string_lossy();

                                        if !level_5_path.is_dir() {
                                            continue;
                                        }

                                        let starts_with_apk = level_5_name.starts_with("APK") || level_5_name.starts_with("apk");
                                        let starts_with_track = level_5_name.starts_with("整轨");
                                        let contains_dynamic = level_5_name.contains("动态");
                                        let contains_static = level_5_name.contains("静态");

                                        let level_5_type = if starts_with_apk {
                                            Some("APK")
                                        } else if starts_with_track {
                                            Some("整轨")
                                        } else {
                                            None
                                        };

                                        if level_5_type.is_none() {
                                            errors.push(format!("层级5命名错误: 目录 '{}' 必须以 'APK'、'apk' 或 '整轨' 开头。", level_5_path.display()));
                                            continue;
                                        }

                                        if !contains_dynamic && !contains_static {
                                            errors.push(format!("层级5命名错误: 目录 '{}' 必须包含 '静态' 或 '动态'。", level_5_path.display()));
                                        }

                                        // 第六层：主驾/副驾/左后/右后
                                        if let Ok(level_6_entries) = fs::read_dir(&level_5_path) {
                                            for level_6_entry in level_6_entries.flatten() {
                                                let level_6_path = level_6_entry.path();
                                                let level_6_name = level_6_path.file_name().unwrap_or_default().to_string_lossy();

                                                if !level_6_path.is_dir() {
                                                    continue;
                                                }

                                                let allowed_names = ["主驾", "副驾", "左后", "右后"];
                                                if !allowed_names.contains(&level_6_name.as_ref()) {
                                                    errors.push(format!("层级6命名错误: 目录 '{}' 名称必须是 '主驾', '副驾', '左后', '右后' 中的一个。", level_6_path.display()));
                                                    continue;
                                                }

                                                // 第七层：内容检查
                                                match level_5_type {
                                                    Some("整轨") => {
                                                        if !check_pcm_files(&level_6_path) {
                                                            errors.push(format!("层级7内容错误 ('整轨'): 目录 '{}' 下必须包含至少一个 .pcm 文件。", level_6_path.display()));
                                                        }
                                                    }
                                                    Some("APK") => {
                                                        let list_txt = level_6_path.join("list.txt");
                                                        let six_test_dir = level_6_path.join("sixTest");

                                                        if !list_txt.is_file() {
                                                            errors.push(format!("层级7内容错误 ('APK'): 目录 '{}' 下必须包含 'list.txt' 文件。", level_6_path.display()));
                                                        }

                                                        if !six_test_dir.is_dir() {
                                                            errors.push(format!("层级7内容错误 ('APK'): 目录 '{}' 下必须包含名为 'sixTest' 的目录。", level_6_path.display()));
                                                        } else if !check_pcm_files(&six_test_dir) {
                                                            errors.push(format!("层级7/8内容错误 ('APK'/'sixTest'): 目录 '{}' 下必须包含至少一个 .pcm 文件。", six_test_dir.display()));
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    errors
}

/// 主函数：程序入口
fn main() {
    // 获取命令行参数作为目录路径
    let args: Vec<String> = env::args().collect();
    let target_directory = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        // 提示用户输入路径
        println!("请输入要质检的根目录路径 (Level 1): ");
        let mut input = String::new();
        if let Err(_) = io::stdin().read_line(&mut input) {
            println!("读取输入失败。");
            return;
        }
        PathBuf::from(input.trim())
    };

    if !target_directory.exists() {
        println!("错误：指定的路径 '{}' 不存在。", target_directory.display());
    } else if !target_directory.is_dir() {
        println!("错误：指定的路径 '{}' 不是一个目录。", target_directory.display());
    } else {
        println!("\n开始对目录 '{}' 进行质检...", target_directory.display());
        let validation_errors = validate_directory_structure(&target_directory);

        if validation_errors.is_empty() {
            println!("\n质检完成：目录结构符合规范。");
        } else {
            println!("\n质检完成：发现以下问题：");
            for (i, error) in validation_errors.iter().enumerate() {
                println!("{}. {}", i + 1, error);
            }
        }

        println!("\n--- 质检结束 ---");
    }
}
