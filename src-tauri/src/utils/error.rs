use tauri::ipc::InvokeError;

/// Result를 InvokeError로 변환하는 트레잇
pub trait IntoInvokeError<T> {
    fn into_invoke_err(self) -> Result<T, InvokeError>;
}

impl<T, E: ToString> IntoInvokeError<T> for Result<T, E> {
    fn into_invoke_err(self) -> Result<T, InvokeError> {
        self.map_err(|e| InvokeError::from(e.to_string()))
    }
}
