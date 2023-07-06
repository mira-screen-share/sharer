use crate::result::Result;

pub trait DisplaySelector {
    type Display: ToString + Eq;

    fn available_displays(&self) -> Result<Vec<Self::Display>>;

    fn select_display(&mut self, display: &Self::Display) -> Result<()>;

    fn selected_display(&self) -> Result<Option<Self::Display>>;
}
