use crate::result::Result;

pub trait Named {
    fn name(&self) -> String;
}

pub trait DisplaySelector {
    type Display: Named + Send;

    fn available_displays(&self) -> Result<Vec<Self::Display>>;

    fn select_display(&mut self, display: &Self::Display) -> Result<()>;
}
