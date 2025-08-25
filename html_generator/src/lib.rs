use std::fmt::{Display, Formatter};

pub struct IsAttribute;
pub struct IsChild;
pub trait AttributeOrChild {
    type IsAttributeOrChild: IsAttributeOrChild;
}

pub trait IsAttributeOrChild {
    type PrependedAttrs<T, O0>;
    type PrependedChildren<T, O1>;
    fn prepend_to_attrs_or_children<T, O0, O1>(t: T, other: (O0, O1)) -> (Self::PrependedAttrs<T, O0>, Self::PrependedChildren<T, O1>);
}
impl IsAttributeOrChild for IsAttribute {
    type PrependedAttrs<T, O0> = (T, O0);
    type PrependedChildren<T, O1> = O1;
    fn prepend_to_attrs_or_children<T, O0, O1>(t: T, (o0, o1): (O0, O1)) -> (Self::PrependedAttrs<T, O0>, Self::PrependedChildren<T, O1>) {
        ((t, o0), o1)
    }
}
impl IsAttributeOrChild for IsChild {
    type PrependedAttrs<T, O0> = O0;
    type PrependedChildren<T, O1> = (T, O1);
    fn prepend_to_attrs_or_children<T, O0, O1>(t: T, (o0, o1): (O0, O1)) -> (Self::PrependedAttrs<T, O0>, Self::PrependedChildren<T, O1>) {
        (o0, (t, o1))
    }
}

impl<StrKey, StrVal> AttributeOrChild for SHtmlAttr<StrKey, StrVal> {
    type IsAttributeOrChild = IsAttribute;
}

impl<Attributes: HtmlAttrs, Children: HtmlChildren> AttributeOrChild for HtmlElement<Attributes, Children> {
    type IsAttributeOrChild = IsChild;
}

impl AttributeOrChild for &str {
    type IsAttributeOrChild = IsChild;
}

impl AttributeOrChild for String {
    type IsAttributeOrChild = IsChild;
}

pub trait AttributesAndChildren {
    type Attributes;
    type Children;
    fn split_into_attributes_and_children(self) -> (Self::Attributes, Self::Children);
}

macro_rules! nest_prepend_to_attrs_or_children(
    ($prepended:ident,) => {
        ()
    };
    ($prepended:ident, $t0:ident $($t:ident)*) => {
        <$t0::IsAttributeOrChild as IsAttributeOrChild>::$prepended<
            $t0,
            nest_prepend_to_attrs_or_children!($prepended, $($t)*)
        >
    };
);
impl<AorC: AttributeOrChild> AttributesAndChildren for AorC {
    type Attributes = nest_prepend_to_attrs_or_children!(PrependedAttrs, AorC);
    type Children = nest_prepend_to_attrs_or_children!(PrependedChildren, AorC);
    fn split_into_attributes_and_children(self) -> (Self::Attributes, Self::Children) {
        <AorC::IsAttributeOrChild as IsAttributeOrChild>::prepend_to_attrs_or_children(self, ((), ()))
    }
}

macro_rules! impl_t_split_into_attrs_and_children {($($t:ident)*) => {
    impl<$($t: AttributesAndChildren,)*> AttributesAndChildren for ($($t,)*) {
        type Attributes = ($($t::Attributes,)*);
        type Children = ($($t::Children,)*);
        fn split_into_attributes_and_children(self) -> (Self::Attributes, Self::Children) {
            #[allow(non_snake_case)]
            let ($($t,)*) = self;
            #[allow(non_snake_case)]
            let ($($t,)*) = ($($t.split_into_attributes_and_children(),)*);
            (($($t.0,)*), ($($t.1,)*))
        }
    }
}}

impl_t_split_into_attrs_and_children!();
impl_t_split_into_attrs_and_children!(T0);
impl_t_split_into_attrs_and_children!(T0 T1);
impl_t_split_into_attrs_and_children!(T0 T1 T2);
impl_t_split_into_attrs_and_children!(T0 T1 T2 T3);
impl_t_split_into_attrs_and_children!(T0 T1 T2 T3 T4);

#[test]
pub fn testme() { // TODO remove this
    dbg!((class("Test"), Some(title("Test2")), div((class("innerdiv"), div(Some(span("test"))), div("test")))).split_into_attributes_and_children());
    dbg!(((class("Test"), class("test2")), Some(title("Test2")), div((class("innerdiv"), div(Some(span("test"))), div("test")))).split_into_attributes_and_children());
}

#[derive(Debug, Clone)]
pub struct HtmlElement<Attributes: HtmlAttrs, Children: HtmlChildren> {
    tag_name: &'static str, // TODO impl Borrow<str>?
    attributes: Attributes,
    children: Children,
}

impl<Attributes: HtmlAttrs, Children: HtmlChildren> HtmlElement<Attributes, Children> {
    pub fn new(tag_name: &'static str, attributes: Attributes, children: Children) -> Self {
        Self{tag_name, attributes, children}
    }
}

macro_rules! impl_element(($tag_name:ident) => {
    pub fn $tag_name<AandC: AttributesAndChildren>(attributes_and_children: AandC) -> HtmlElement<AandC::Attributes, AandC::Children>
        where
            AandC::Attributes: HtmlAttrs,
            AandC::Children: HtmlChildren,
    {
        let (attributes, children) = attributes_and_children.split_into_attributes_and_children();
        HtmlElement::new(
            stringify!($tag_name),
            attributes,
            children,
        )
    }
});

impl_element!(table);
impl_element!(tbody);
impl_element!(tr);
impl_element!(th);
impl_element!(td);
impl_element!(div);
impl_element!(span);

pub trait HtmlAttrs {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error>;
}
pub trait HtmlChildren {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error>;
}
impl<Attributes: HtmlAttrs, Children: HtmlChildren> Display for HtmlElement<Attributes, Children> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let tag_name = self.tag_name;
        write!(formatter, "<{tag_name}")?;
        self.attributes.fmt_attrs(formatter)?;
        write!(formatter, ">")?;
        self.children.fmt_children(formatter)?;
        write!(formatter, "</{tag_name}>")?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SHtmlAttr<StrKey, StrVal>(StrKey, StrVal);
impl<StrKey: std::borrow::Borrow<str>, StrVal: std::borrow::Borrow<str>> HtmlAttrs for SHtmlAttr<StrKey, StrVal> {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(formatter, " {}=\"{}\"", self.0.borrow(), self.1.borrow())
    }
}

macro_rules! impl_attr(($attr:ident) => {
    pub fn $attr<StrVal: std::borrow::Borrow<str>>(str_val: StrVal) -> SHtmlAttr<&'static str, StrVal> {
        SHtmlAttr(stringify!($attr), str_val)
    }
});
impl_attr!(class);
impl_attr!(title);
impl_attr!(style);
impl_attr!(colspan);

impl<const N: usize, StrName: std::borrow::Borrow<str>, StrVal: std::borrow::Borrow<str>> HtmlAttrs for [(StrName, StrVal); N] {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for (str_name, str_val) in self {
            write!(formatter, " {}=\"{}\"", str_name.borrow(), str_val.borrow())?;
        }
        Ok(())
    }
}
impl<Attributes: HtmlAttrs, Children: HtmlChildren> HtmlChildren for HtmlElement<Attributes, Children> {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}
impl HtmlChildren for &str {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}
impl HtmlChildren for String {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}
macro_rules! impl_html_attrs_and_children_for_tuple{($($tuple_component:ident)*) => {
    impl<$($tuple_component: HtmlAttrs,)*> HtmlAttrs for ($($tuple_component,)*) {
        #[allow(unused_variables)]
        fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            #[allow(non_snake_case)]
            let ($($tuple_component,)*) = self;
            $($tuple_component.fmt_attrs(formatter)?;)*
            Ok(())
        }
    }
    impl<$($tuple_component: HtmlChildren,)*> HtmlChildren for ($($tuple_component,)*) {
        #[allow(unused_variables)]
        fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            #[allow(non_snake_case)]
            let ($($tuple_component,)*) = self;
            $($tuple_component.fmt_children(formatter)?;)*
            Ok(())
        }
    }
}}
impl_html_attrs_and_children_for_tuple!();
impl_html_attrs_and_children_for_tuple!(T0);
impl_html_attrs_and_children_for_tuple!(T0 T1);
impl_html_attrs_and_children_for_tuple!(T0 T1 T2);
impl_html_attrs_and_children_for_tuple!(T0 T1 T2 T3);
impl_html_attrs_and_children_for_tuple!(T0 T1 T2 T3 T4);

impl<Attributes: HtmlAttrs> HtmlAttrs for Vec<Attributes> {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for htmlattr in self {
            htmlattr.fmt_attrs(formatter)?;
        }
        Ok(())
    }
}
impl<Children: HtmlChildren> HtmlChildren for Vec<Children> {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for htmlchild in self {
            htmlchild.fmt_children(formatter)?;
        }
        Ok(())
    }
}
impl<T: AttributeOrChild> AttributeOrChild for Vec<T> {
    type IsAttributeOrChild = T::IsAttributeOrChild;
}
impl<Attributes: HtmlAttrs> HtmlAttrs for Option<Attributes> {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Some(htmlattr) = self {
            htmlattr.fmt_attrs(formatter)?;
        }
        Ok(())
    }
}
impl<Children: HtmlChildren> HtmlChildren for Option<Children> {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Some(htmlchild) = self {
            htmlchild.fmt_children(formatter)?;
        }
        Ok(())
    }
}
impl<T: AttributeOrChild> AttributeOrChild for Option<T> {
    type IsAttributeOrChild = T::IsAttributeOrChild;
}

pub fn html_iter<Iter>(it: Iter) -> impl HtmlChildren + AttributeOrChild<IsAttributeOrChild=<Iter::Item as AttributeOrChild>::IsAttributeOrChild>
    where
        Iter: Iterator+Clone,
        Iter::Item: HtmlChildren + AttributeOrChild,
{
    struct SHtmlChildrenIterator<Iter>(Iter);
    impl<Iter> HtmlChildren for SHtmlChildrenIterator<Iter>
        where
            Iter: Iterator+Clone,
            Iter::Item: HtmlChildren + AttributeOrChild,
    {
        fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            for htmlchild in self.0.clone() {
                htmlchild.fmt_children(formatter)?;
            }
            Ok(())
        }
    }
    impl<Iter> AttributeOrChild for SHtmlChildrenIterator<Iter>
        where
            Iter: Iterator+Clone,
            Iter::Item: HtmlChildren + AttributeOrChild,
    {
        type IsAttributeOrChild = <Iter::Item as AttributeOrChild>::IsAttributeOrChild;
    }
    SHtmlChildrenIterator(it)
}
