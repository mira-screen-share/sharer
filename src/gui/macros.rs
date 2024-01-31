#[macro_export]
macro_rules! column_iced {
    () => (
        iced::widget::Column::new()
    );
    ($($x:expr),+ $(,)?) => (
        iced::widget::Column::with_children(vec![$(iced::Element::from($x)),+])
    );
}

#[allow(unused_imports)]
pub use column_iced;
