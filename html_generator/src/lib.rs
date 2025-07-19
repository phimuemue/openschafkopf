use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct SHtmlElement<Attrs, Children> {
    str_tag_name: &'static str, // TODO impl Borrow<str>?
    attrs: Attrs,
    children: Children,
}

impl<Attrs, Children> SHtmlElement<Attrs, Children> {
    pub fn new(str_tag_name: &'static str, attrs: Attrs, children: Children) -> Self {
        Self{str_tag_name, attrs, children}
    }
}

macro_rules! impl_element(($tag_name:ident) => {
    pub fn $tag_name<Children>(children: Children) -> SHtmlElement</*Attrs*/(), Children> {
        $tag_name::with_attrs(/*attrs*/(), children)
        
    }
    pub mod $tag_name {
        use super::SHtmlElement;
        pub fn with_attrs<Attrs, Children>(attrs: Attrs, children: Children) -> SHtmlElement<Attrs, Children> {
            SHtmlElement::new(stringify!($tag_name), attrs, children)
        }
    }
});

impl_element!(table);
impl_element!(tbody);
impl_element!(tr);
impl_element!(th);
impl_element!(td);
impl_element!(div);
impl_element!(span);

pub trait THtmlAttrs {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error>;
}
pub trait THtmlChildren {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error>;
}
impl<Attrs: THtmlAttrs, Children: THtmlChildren> Display for SHtmlElement<Attrs, Children> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let str_tag_name = self.str_tag_name;
        write!(formatter, "<{str_tag_name}")?;
        self.attrs.fmt_attrs(formatter)?;
        write!(formatter, ">")?;
        self.children.fmt_children(formatter)?;
        write!(formatter, "</{str_tag_name}>")?;
        Ok(())
    }
}

impl THtmlAttrs for () {
    fn fmt_attrs(&self, _formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Ok(())
    }
}
impl<StrName: std::borrow::Borrow<str>, StrVal: std::borrow::Borrow<str>> THtmlAttrs for Vec<(StrName, StrVal)> {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for (str_name, str_val) in self {
            write!(formatter, " {}{}", str_name.borrow(), '=')?;
            write!(formatter, "{}", str_val.borrow())?;
        }
        Ok(())
    }
}
impl<const N: usize, StrName: std::borrow::Borrow<str>, StrVal: std::borrow::Borrow<str>> THtmlAttrs for [(StrName, StrVal); N] {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for (str_name, str_val) in self {
            write!(formatter, " {}{}", str_name.borrow(), '=')?;
            write!(formatter, "{}", str_val.borrow())?;
        }
        Ok(())
    }
}
impl THtmlChildren for () {
    fn fmt_children(&self, _formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Ok(())
    }
}
impl<Attrs: THtmlAttrs, Children: THtmlChildren> THtmlChildren for SHtmlElement<Attrs, Children> {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}
impl<'str> THtmlChildren for &'str str {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}
impl THtmlChildren for String {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}
impl<HtmlChildren0: THtmlChildren> THtmlChildren for (HtmlChildren0,) {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.0.fmt_children(formatter)?;
        Ok(())
    }
}
impl<HtmlChildren0: THtmlChildren, HtmlChildren1: THtmlChildren> THtmlChildren for (HtmlChildren0, HtmlChildren1) {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.0.fmt_children(formatter)?;
        self.1.fmt_children(formatter)?;
        Ok(())
    }
}
impl<HtmlChildren0: THtmlChildren, HtmlChildren1: THtmlChildren, HtmlChildren2: THtmlChildren> THtmlChildren for (HtmlChildren0, HtmlChildren1, HtmlChildren2) {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.0.fmt_children(formatter)?;
        self.1.fmt_children(formatter)?;
        self.2.fmt_children(formatter)?;
        Ok(())
    }
}
impl<HtmlChildren0: THtmlChildren> THtmlChildren for Vec<HtmlChildren0> {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for htmlchild in self {
            htmlchild.fmt_children(formatter)?;
        }
        Ok(())
    }
}
impl<HtmlChildren0: THtmlChildren> THtmlChildren for Option<HtmlChildren0> {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Some(htmlchild) = self {
            htmlchild.fmt_children(formatter)?;
        }
        Ok(())
    }
}

