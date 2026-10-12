// codex.rs — 调用本机 codex CLI：截图+诉求 → 回答；多轮续聊、中途加图自动降级
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

fn transcript() -> &'static Mutex<Vec<Value>> {
    static T: OnceLock<Mutex<Vec<Value>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(Vec::new()))
}

/// 前端每轮把对话镜像过来（role/text，最多留 8 条），供降级续聊拼摘要
#[tauri::command]
pub fn codex_mirror_transcript(rounds: Vec<Value>) -> Result<Value, String> {
    let mut g = transcript().lock().unwrap();
    g.clear();
    g.extend(rounds.into_iter().take(8));
    Ok(json!({"ok": true}))
}

#[tauri::command]
pub fn codex_send(
    app: AppHandle,
    prompt: String,
    images: Vec<String>,
    resume: bool,
) -> Result<Value, String> {
    if prompt.trim().is_empty() {
        return Err("问题为空".into());
    }
    if !codex_exists() {
        return Err(
            "未找到 codex 命令。请先安装并登录：npm i -g @openai/codex，然后终端运行 codex 完成登录"
                .into(),
        );
    }

    std::thread::spawn(move || {
        if resume {
            // 优先 codex exec resume --last；带图或失败时降级为新线程 + 摘要
            let mut args = vec!["exec".to_string(), "resume".into(), "--last".into()];
            for p in &images {
                args.push("-i".into());
                args.push(p.clone());
            }
            args.push(prompt.clone());
            if run_codex(&app, &args) == 0 {
                return;
            }
            if !images.is_empty() {
                let _ = app.emit(
                    "codex:note",
                    json!({"text": "续聊携带图片未成功，已自动开新线程并带上对话摘要"}),
                );
            }
        }

        // 全新线程（或降级路径）
        let mut args = vec!["exec".to_string(), "--skip-git-repo-check".into()];
        for p in &images {
            args.push("-i".into());
            args.push(p.clone());
        }
        if resume {
            let recap = recent_transcript();
            let recap = if recap.is_empty() {
                String::new()
            } else {
                format!("（此前对话摘要）\n{recap}\n（摘要结束）\n\n")
            };
            args.push(format!("{recap}新问题：{prompt}"));
        } else {
            args.push(prompt.clone());
        }
        run_codex(&app, &args);
    });
    Ok(json!({"ok": true}))
}

fn codex_exists() -> bool {
    #[cfg(windows)]
    let probe = Command::new("where").arg("codex").output();
    #[cfg(not(windows))]
    let probe = Command::new("sh").arg("-c").arg("command -v codex").output();
    probe.map(|o| o.status.success()).unwrap_or(false)
}

/// 逐行流式推送 stdout；返回退出码（失败 None）
fn run_codex(app: &AppHandle, args: &[String]) -> i32 {
    let _ = app.emit("codex:start", json!({}));
    let spawned = Command::new("codex")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => {
            let _ = app.emit(
                "codex:done",
                json!({"code": -1, "error": format!("启动 codex 失败: {e}")}),
            );
            return -1;
        }
    };

    if let Some(out) = child.stdout.take() {
        for (seq, line) in BufReader::new(out)
            .lines()
            .map_while(|l| l.ok())
            .enumerate()
        {
            let _ = app.emit("codex:delta", json!({"seq": seq, "text": line}));
        }
    }
    let mut err_tail = String::new();
    if let Some(err) = child.stderr.take() {
        for line in BufReader::new(err).lines().map_while(|l| l.ok()) {
            err_tail = line;
        }
    }
    match child.wait() {
        Ok(st) => {
            let code = st.code().unwrap_or(-1);
            let _ = app.emit(
                "codex:done",
                if code == 0 {
                    json!({"code": 0})
                } else {
                    json!({"code": code, "error": err_tail})
                },
            );
            code
        }
        Err(e) => {
            let _ = app.emit("codex:done", json!({"code": -1, "error": e.to_string()}));
            -1
        }
    }
}

fn recent_transcript() -> String {
    transcript()
        .lock()
        .unwrap()
        .iter()
        .map(|r| {
            let role = r.get("role").and_then(|v| v.as_str()).unwrap_or("?");
            let text = r.get("text").and_then(|v| v.as_str()).unwrap_or("");
            format!("[{role}] {text}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
