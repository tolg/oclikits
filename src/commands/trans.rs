use crate::cli::TransArgs;
use crate::error::Result;
use crate::i18n::is_zh;
use crate::opencode::runner::{RunOptions, Runner};
use crate::opencode::{resolve_model, Command};
use crate::ui::CommandTimer;

/// `oct` — Chinese/English translation with spelling correction.
///
/// Every translation uses a fresh ephemeral session, so context never leaks
/// between translations.
pub async fn execute(args: TransArgs) -> Result<()> {
    let timer = CommandTimer::start();

    let text = args.text.join(" ");
    let text = text.trim();
    if text.is_empty() {
        print_usage();
        return Ok(());
    }

    let model = resolve_model(Command::Trans, &args.model)?;
    let runner = Runner::new(args.debug);
    let options = RunOptions {
        model,
        session_id: None,
        directory: None,
        prompt: build_prompt(text),
        ephemeral: true,
        show_tool_calls: false,
    };

    runner.run(&options).await?;

    timer.print_elapsed();
    Ok(())
}

fn build_prompt(text: &str) -> String {
    let (rule14_header, rule14_fix, rule16_header) = if is_zh() {
        (
            "您是否想查询以下句子？",
            "修正:",
            "您是否想查询 \\\"{correct_word}\\\"？",
        )
    } else {
        (
            "Did you mean the following sentence?",
            "Correction:",
            "Did you mean \\\"{correct_word}\\\"?",
        )
    };

    let rules = format!(
        "You are a strict Chinese-English translator.\n\
Rules:\n\
1. Detect whether the input is primarily Chinese or English, then translate it into the other language.\n\
2. If the input is a single word, output 3 to 6 concise translation candidates, one per line.\n\
3. If the input is a sentence, phrase, or multi-word expression, output exactly one best translation.\n\
4. Output only the translation result. Do not explain, label, number, or add notes.\n\
5. Preserve meaning, tone, and common usage. Prefer natural modern language.\n\
6. For single-word translation, return only common, standard, high-frequency equivalents used in modern everyday language.\n\
7. Do not output rare, archaic, literary, domain-specific, brand-like, transliterated, speculative, or awkward candidates.\n\
8. If the source word has fewer than 3 truly common equivalents, return only the common ones. Quality is more important than count.\n\
9. Avoid near-duplicate variants that differ only slightly in wording.\n\
10. If the input is an English special term, proper noun, style name, product name, or community term with no direct standard Chinese translation, output concise Chinese explanations instead of English paraphrases.\n\
11. In that case, every output line must be Chinese, except the original term itself when it is necessary to identify the subject.\n\
12. Never respond with an English definition when the target language should be Chinese.\n\
13. If the input is an English sentence or phrase with spelling or grammar errors, first output the corrected text, then provide its Chinese translation.\n\
14. Output format for corrected sentence: first line is exactly → {}, second line is exactly {} {{corrected_sentence}}, followed by a blank line, then the translation (one line as per rule 3 for sentences).\n\
15. If the input is a single English word that appears to be misspelled, first output the correction hint, then provide its Chinese translation as usual.\n\
16. Output format for corrected single word: first line is exactly → {}, followed by a blank line, then the translation candidates one per line.\n\
17. Only apply spelling/grammar correction to English input. Do not apply to Chinese input or proper nouns/terms that look intentional.\n\
18. Do not call any tools or read any files. Reply with the translation result only.",
        rule14_header, rule14_fix, rule16_header
    );

    format!("{}\n\nInput:\n{}", rules, text)
}

fn print_usage() {
    if is_zh() {
        println!("用法: oct [选项] <翻译内容>");
        println!("  -m, --model       覆盖配置文件中的模型");
        println!("  --debug           输出原始 OpenCode 事件");
    } else {
        println!("Usage: oct [options] <text>");
        println!("  -m, --model       Override the model from config.jsonc");
        println!("  --debug           Print raw OpenCode events");
    }
}
