#[macro_export]
macro_rules! column_iced {
    () => (
        iced::widget::Column::new()
    );
    ($($x:expr),+ $(,)?) => (
        iced::widget::Column::with_children(vec![$(iced::Element::from($x)),+])
    );
}

pub use column_iced;
