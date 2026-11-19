// Modified by the Wordsmith Script contributors.
use super::{Arc, DocLayout, LayoutCache, LayoutOptions, Session, ViewMode};

impl Session {
    /// The current layout (recomputed when the document or view changed). Paragraph layouts
    /// are cached, so an edit lays out only the paragraphs it changed.
    pub fn layout(&mut self) -> Arc<DocLayout> {
        let mode = self.view_mode;
        let width = self.view_width;
        let rev = self.rev;
        let fits = |e: &Option<super::snapshot_types::LayoutEntry>| {
            e.as_ref().is_some_and(|(r, w, m, _)| *r == rev && (*w == width || mode == ViewMode::Print) && *m == mode)
        };
        if !fits(&self.layout[0]) && fits(&self.layout[1]) {
            self.layout.swap(0, 1);
        }
        if fits(&self.layout[0])
            && let Some(e) = &self.layout[0]
        {
            return e.3.clone();
        }
        let opts = LayoutOptions { view: mode, web_width: width, ..Default::default() };
        let l = Arc::new(wordsmith_layout::layout(&self.doc, &mut self.cache, &opts));
        let [newest, _] = std::mem::take(&mut self.layout);
        self.layout = [Some((rev, width, mode, l.clone())), newest];
        l
    }

    /// A layout for output (PDF, print): print view, no proofing marks, page references in the
    /// body showing their bookmarks' pages.
    pub fn export_layout(&self) -> Arc<DocLayout> {
        let opts = LayoutOptions { view: ViewMode::Print, output_page_refs: true, ..Default::default() };
        Arc::new(wordsmith_layout::layout(&self.doc, &mut LayoutCache::new(), &opts))
    }

    /// Invalidate the cached layout (fonts changed etc.).
    pub fn relayout(&mut self) {
        self.layout = Default::default();
        self.cache.clear();
    }
}
