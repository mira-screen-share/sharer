use crate::result::Result;

pub trait DisplaySelector {
    type Display: ToString + Eq + Send;

    fn available_displays(&mut self) -> Result<Vec<Self::Display>>;

    fn select_display(&mut self, display: &Self::Display) -> Result<()>;

    fn selected_display(&self) -> Result<Option<Self::Display>>;
}
