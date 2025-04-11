use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn write_to_user(s: &str) {
    let mut stdout = tokio::io::stdout();
    let _ = stdout.write(s.as_bytes()).await;
    let _ = stdout.flush().await;
}

pub enum UserInput {
    Yes,
    No,
    NotExpectedInput,
    InvalidInput,
}

pub async fn take_user_input() -> Result<String, std::string::FromUtf8Error> {
    let mut user_input = [0; 128];
    let mut stdin = tokio::io::stdin();
    let bytes = stdin.read(&mut user_input).await.unwrap();
    let input = user_input[..bytes].trim_ascii_end();
    String::from_utf8(input.to_vec())
}

pub async fn user_input_yes_or_no() -> UserInput {
    let Ok(user_input) = take_user_input().await else {
        return UserInput::InvalidInput;
    };

    match user_input.as_str() {
        "y" | "Y" | "Yes" | "yes" | "YES" => UserInput::Yes,
        "n" | "N" | "No" | "no" | "NO" => UserInput::No,
        _ => UserInput::NotExpectedInput,
    }
}
