use std::fmt::{Display, Formatter};

pub struct SPrependToAttrs;
pub struct SPrependToChildren;
pub trait TIntoPrependToAttrsOrChildren {
    type PrependToAttrsOrChildren: TPrependToAttrsOrChildren;
}

pub trait TPrependToAttrsOrChildren {
    type PrependedAttrs<T, O0>;
    type PrependedChildren<T, O1>;
    fn prepend_to_attrs_or_children<T, O0, O1>(t: T, other: (O0, O1)) -> (Self::PrependedAttrs<T, O0>, Self::PrependedChildren<T, O1>);
}
impl TPrependToAttrsOrChildren for SPrependToAttrs {
    type PrependedAttrs<T, O0> = (T, O0);
    type PrependedChildren<T, O1> = O1;
    fn prepend_to_attrs_or_children<T, O0, O1>(t: T, (o0, o1): (O0, O1)) -> (Self::PrependedAttrs<T, O0>, Self::PrependedChildren<T, O1>) {
        ((t, o0), o1)
    }
}
impl TPrependToAttrsOrChildren for SPrependToChildren {
    type PrependedAttrs<T, O0> = O0;
    type PrependedChildren<T, O1> = (T, O1);
    fn prepend_to_attrs_or_children<T, O0, O1>(t: T, (o0, o1): (O0, O1)) -> (Self::PrependedAttrs<T, O0>, Self::PrependedChildren<T, O1>) {
        (o0, (t, o1))
    }
}

impl<StrKey, StrVal> TIntoPrependToAttrsOrChildren for SHtmlAttr<StrKey, StrVal> {
    type PrependToAttrsOrChildren = SPrependToAttrs;
}

impl<Attrs: THtmlAttrs, Children: THtmlChildren> TIntoPrependToAttrsOrChildren for SHtmlElement<Attrs, Children> {
    type PrependToAttrsOrChildren = SPrependToChildren;
}

impl TIntoPrependToAttrsOrChildren for &str {
    type PrependToAttrsOrChildren = SPrependToChildren;
}

impl TIntoPrependToAttrsOrChildren for String {
    type PrependToAttrsOrChildren = SPrependToChildren;
}

pub trait TSplitIntoAttrsAndChildren {
    type Attrs;
    type Children;
    fn split_into_attrs_and_children(self) -> (Self::Attrs, Self::Children);
}

macro_rules! nest_prepend_to_attrs_or_children(
    ($prepended:ident,) => {
        ()
    };
    ($prepended:ident, $t0:ident $($t:ident)*) => {
        <$t0::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::$prepended<
            $t0,
            nest_prepend_to_attrs_or_children!($prepended, $($t)*)
        >
    };
);
impl<IntoPrependToAttrsOrChildren: TIntoPrependToAttrsOrChildren> TSplitIntoAttrsAndChildren for IntoPrependToAttrsOrChildren {
    type Attrs = nest_prepend_to_attrs_or_children!(PrependedAttrs, IntoPrependToAttrsOrChildren);
    type Children = nest_prepend_to_attrs_or_children!(PrependedChildren, IntoPrependToAttrsOrChildren);
    fn split_into_attrs_and_children(self) -> (Self::Attrs, Self::Children) {
        <IntoPrependToAttrsOrChildren::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::prepend_to_attrs_or_children(self, ((), ()))
    }
}

macro_rules! impl_t_split_into_attrs_and_children {($($t:ident)*) => {
    impl<$($t: TSplitIntoAttrsAndChildren,)*> TSplitIntoAttrsAndChildren for ($($t,)*) {
        type Attrs = ($($t::Attrs,)*);
        type Children = ($($t::Children,)*);
        fn split_into_attrs_and_children(self) -> (Self::Attrs, Self::Children) {
            #[allow(non_snake_case)]
            let ($($t,)*) = self;
            #[allow(non_snake_case)]
            let ($($t,)*) = ($($t.split_into_attrs_and_children(),)*);
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
    dbg!((class("Test"), Some(title("Test2")), div((class("innerdiv"), div(Some(span("test"))), div("test")))).split_into_attrs_and_children());
    dbg!(((class("Test"), class("test2")), Some(title("Test2")), div((class("innerdiv"), div(Some(span("test"))), div("test")))).split_into_attrs_and_children());
}

#[derive(Debug, Clone)]
pub struct SHtmlElement<Attrs: THtmlAttrs, Children: THtmlChildren> {
    str_tag_name: &'static str, // TODO impl Borrow<str>?
    attrs: Attrs,
    children: Children,
}

impl<Attrs: THtmlAttrs, Children: THtmlChildren> SHtmlElement<Attrs, Children> {
    pub fn new(str_tag_name: &'static str, attrs: Attrs, children: Children) -> Self {
        Self{str_tag_name, attrs, children}
    }
}

macro_rules! impl_element(($tag_name:ident) => {
    pub fn $tag_name<MakeAttrsAndChildren: TSplitIntoAttrsAndChildren>(makeattrsandchildren: MakeAttrsAndChildren) -> SHtmlElement<MakeAttrsAndChildren::Attrs, MakeAttrsAndChildren::Children>
        where
            MakeAttrsAndChildren::Attrs: THtmlAttrs,
            MakeAttrsAndChildren::Children: THtmlChildren,
    {
        let (attrs, children) = makeattrsandchildren.split_into_attrs_and_children();
        SHtmlElement::new(
            stringify!($tag_name),
            attrs,
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

#[derive(Debug, Clone)]
pub struct SHtmlAttr<StrKey, StrVal>(StrKey, StrVal);
impl<StrKey: std::borrow::Borrow<str>, StrVal: std::borrow::Borrow<str>> THtmlAttrs for SHtmlAttr<StrKey, StrVal> {
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

impl<const N: usize, StrName: std::borrow::Borrow<str>, StrVal: std::borrow::Borrow<str>> THtmlAttrs for [(StrName, StrVal); N] {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for (str_name, str_val) in self {
            write!(formatter, " {}=\"{}\"", str_name.borrow(), str_val.borrow())?;
        }
        Ok(())
    }
}
impl<Attrs: THtmlAttrs, Children: THtmlChildren> THtmlChildren for SHtmlElement<Attrs, Children> {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}
impl THtmlChildren for &str {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}
impl THtmlChildren for String {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}
macro_rules! impl_html_attrs_and_children_for_tuple{($($tuple_component:ident)*) => {
    impl<$($tuple_component: THtmlAttrs,)*> THtmlAttrs for ($($tuple_component,)*) {
        #[allow(unused_variables)]
        fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            #[allow(non_snake_case)]
            let ($($tuple_component,)*) = self;
            $($tuple_component.fmt_attrs(formatter)?;)*
            Ok(())
        }
    }
    impl<$($tuple_component: THtmlChildren,)*> THtmlChildren for ($($tuple_component,)*) {
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

impl<HtmlAttrs: THtmlAttrs> THtmlAttrs for Vec<HtmlAttrs> {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for htmlattr in self {
            htmlattr.fmt_attrs(formatter)?;
        }
        Ok(())
    }
}
impl<HtmlChildren: THtmlChildren> THtmlChildren for Vec<HtmlChildren> {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for htmlchild in self {
            htmlchild.fmt_children(formatter)?;
        }
        Ok(())
    }
}
impl<T: TIntoPrependToAttrsOrChildren> TIntoPrependToAttrsOrChildren for Vec<T> {
    type PrependToAttrsOrChildren = T::PrependToAttrsOrChildren;
}
impl<HtmlAttrs: THtmlAttrs> THtmlAttrs for Option<HtmlAttrs> {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Some(htmlattr) = self {
            htmlattr.fmt_attrs(formatter)?;
        }
        Ok(())
    }
}
impl<HtmlChildren: THtmlChildren> THtmlChildren for Option<HtmlChildren> {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Some(htmlchild) = self {
            htmlchild.fmt_children(formatter)?;
        }
        Ok(())
    }
}
impl<T: TIntoPrependToAttrsOrChildren> TIntoPrependToAttrsOrChildren for Option<T> {
    type PrependToAttrsOrChildren = T::PrependToAttrsOrChildren;
}

pub fn html_iter<Iter>(it: Iter) -> impl THtmlChildren + TIntoPrependToAttrsOrChildren<PrependToAttrsOrChildren=<Iter::Item as TIntoPrependToAttrsOrChildren>::PrependToAttrsOrChildren>
    where
        Iter: Iterator+Clone,
        Iter::Item: THtmlChildren + TIntoPrependToAttrsOrChildren,
{
    struct SHtmlChildrenIterator<Iter>(Iter);
    impl<Iter> THtmlChildren for SHtmlChildrenIterator<Iter>
        where
            Iter: Iterator+Clone,
            Iter::Item: THtmlChildren + TIntoPrependToAttrsOrChildren,
    {
        fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            for htmlchild in self.0.clone() {
                htmlchild.fmt_children(formatter)?;
            }
            Ok(())
        }
    }
    impl<Iter> TIntoPrependToAttrsOrChildren for SHtmlChildrenIterator<Iter>
        where
            Iter: Iterator+Clone,
            Iter::Item: THtmlChildren + TIntoPrependToAttrsOrChildren,
    {
        type PrependToAttrsOrChildren = <Iter::Item as TIntoPrependToAttrsOrChildren>::PrependToAttrsOrChildren;
    }
    SHtmlChildrenIterator(it)
}
