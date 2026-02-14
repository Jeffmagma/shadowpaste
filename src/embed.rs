use fastembed::{EmbeddingModel, ImageEmbedding, ImageEmbeddingModel, ImageInitOptions, InitOptions, TextEmbedding};

pub type Embedding = Vec<f32>;

pub struct Embedder {
	text_model: TextEmbedding,
	image_model: ImageEmbedding,
}

impl Embedder {
	pub fn new() -> anyhow::Result<Self> {
		let text_model = TextEmbedding::try_new(
			InitOptions::new(EmbeddingModel::NomicEmbedTextV15).with_show_download_progress(true),
		)?;

		let image_model = ImageEmbedding::try_new(
			ImageInitOptions::new(ImageEmbeddingModel::NomicEmbedVisionV15).with_show_download_progress(true),
		)?;

		Ok(Self {
			text_model,
			image_model,
		})
	}

	/// embed copied text, add `search_document:` prefix
	pub fn embed_document(&mut self, text: &str) -> anyhow::Result<Embedding> {
		let prefixed = format!("search_document: {text}");
		let embeddings = self.text_model.embed(vec![&prefixed], None)?;
		embeddings.into_iter().next().ok_or_else(|| anyhow::anyhow!("No embedding returned"))
	}

	/// embed search query text, add `search_query:` prefix
	pub fn embed_query(&mut self, text: &str) -> anyhow::Result<Embedding> {
		let prefixed = format!("search_query: {text}");
		let embeddings = self.text_model.embed(vec![&prefixed], None)?;
		embeddings.into_iter().next().ok_or_else(|| anyhow::anyhow!("No embedding returned"))
	}

	/// embed image bytes
	pub fn embed_image_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<Embedding> {
		let embeddings = self.image_model.embed_bytes(&[bytes], None)?;
		embeddings.into_iter().next().ok_or_else(|| anyhow::anyhow!("No embedding returned"))
	}

	/// cosine similarity
	pub fn similarity(a: &[f32], b: &[f32]) -> f32 {
		let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
		let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
		let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
		if norm_a == 0.0 || norm_b == 0.0 {
			return 0.0;
		}
		dot / (norm_a * norm_b)
	}
}