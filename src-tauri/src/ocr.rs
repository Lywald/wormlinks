use windows::Media::Ocr::OcrEngine;
use windows::Graphics::Imaging::SoftwareBitmap;

pub struct NativeOcrEngine {
    engine: OcrEngine,
}

impl NativeOcrEngine {
    pub fn new() -> anyhow::Result<Self> {
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
        Ok(Self { engine })
    }

    pub async fn recognize_text(&self, bitmap: SoftwareBitmap) -> anyhow::Result<String> {
        // In windows v0.62+, we can directly await WinRT async operations
        let result = self.engine.RecognizeAsync(&bitmap)?.await?;
        Ok(result.Text()?.to_string())
    }
}
