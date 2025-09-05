use std::fmt::{Display, Formatter};

pub struct IsAttribute;
pub struct IsChild;
pub trait AttributeOrChild {
    type Attribute: AttributeOrChild;
    type Child: AttributeOrChild;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child);
    fn fmt_attr(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error>;
    fn fmt_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error>;
}

impl AttributeOrChild for () {
    type Attribute = ();
    type Child = ();
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        ((), ())
    }
    fn fmt_attr(&self, _formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Ok(())
    }
    fn fmt_child(&self, _formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Ok(())
    }
}

macro_rules! impl_attributeorchild_for_tuple {($($t:ident)*) => {
    impl<$($t: AttributeOrChild,)*> AttributeOrChild for ($($t,)*) {
        type Attribute = ($($t::Attribute,)*);
        type Child = ($($t::Child,)*);
        fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
            #[allow(non_snake_case)]
            let ($($t,)*) = self;
            #[allow(non_snake_case)]
            let ($($t,)*) = ($($t.split_into_attributes_and_children(),)*);
            (
                ($($t.0,)*),
                ($($t.1,)*),
            )
        }
        #[allow(unused_variables)]
        fn fmt_attr(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            #[allow(non_snake_case)]
            let ($($t,)*) = self;
            $($t.fmt_attr(formatter)?;)*
            Ok(())
        }
        #[allow(unused_variables)]
        fn fmt_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            #[allow(non_snake_case)]
            let ($($t,)*) = self;
            $($t.fmt_child(formatter)?;)*
            Ok(())
        }
    }
}}
impl_attributeorchild_for_tuple!(T0);
impl_attributeorchild_for_tuple!(T0 T1);
impl_attributeorchild_for_tuple!(T0 T1 T2);
impl_attributeorchild_for_tuple!(T0 T1 T2 T3);
impl_attributeorchild_for_tuple!(T0 T1 T2 T3 T4);
impl_attributeorchild_for_tuple!(T0 T1 T2 T3 T4 T5);
impl_attributeorchild_for_tuple!(T0 T1 T2 T3 T4 T5 T6);
impl_attributeorchild_for_tuple!(T0 T1 T2 T3 T4 T5 T6 T7);
impl_attributeorchild_for_tuple!(T0 T1 T2 T3 T4 T5 T6 T7 T8);


impl<StrKey: std::borrow::Borrow<str>, StrVal: std::borrow::Borrow<str>> AttributeOrChild for SHtmlAttr<StrKey, StrVal> {
    type Attribute = Self;
    type Child = ();
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        (self, ())
    }
    fn fmt_attr(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(formatter, " {}=\"{}\"", self.0.borrow(), self.1.borrow())
    }
    fn fmt_child(&self, _formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Ok(())
    }
}

impl<AoC: AttributeOrChild> AttributeOrChild for HtmlElement<AoC> {
    type Attribute = ();
    type Child = Self;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        ((), self)
    }
    fn fmt_attr(&self, _formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Ok(())
    }
    fn fmt_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let tag_name = self.tag_name;
        write!(formatter, "<{tag_name}")?;
        self.attributes.fmt_attr(formatter)?;
        write!(formatter, ">")?;
        self.children.fmt_child(formatter)?;
        write!(formatter, "</{tag_name}>")?;
        Ok(())
    }
}
impl<AoC: AttributeOrChild> AttributeOrChild for VoidElement<AoC> {
    type Attribute = ();
    type Child = Self;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        ((), self)
    }
    fn fmt_attr(&self, _formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Ok(())
    }
    fn fmt_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let tag_name = self.tag_name;
        write!(formatter, "<{tag_name}")?;
        self.attributes.fmt_attr(formatter)?;
        write!(formatter, "/>")?;
        Ok(())
    }
}

impl AttributeOrChild for &str {
    type Attribute = ();
    type Child = Self;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        ((), self)
    }
    fn fmt_attr(&self, _formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Ok(())
    }
    fn fmt_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}

impl AttributeOrChild for String {
    type Attribute = ();
    type Child = Self;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        ((), self)
    }
    fn fmt_attr(&self, _formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Ok(())
    }
    fn fmt_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}
impl<'a> AttributeOrChild for std::fmt::Arguments<'a> {
    type Attribute = ();
    type Child = Self;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        ((), self)
    }
    fn fmt_attr(&self, _formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Ok(())
    }
    fn fmt_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}

#[test]
pub fn testme() { // TODO remove this
    dbg!((class("Test"), Some(attributes::title("Test2")), div((class("innerdiv"), div(Some(elements::span("test"))), div("test")))).split_into_attributes_and_children());
    dbg!(((class("Test"), class("test2")), Some(attributes::title("Test2")), div((class("innerdiv"), div(Some(elements::span("test"))), div("test")))).split_into_attributes_and_children());
    dbg!(((class("Test"), class("test2")), Some(attributes::title("Test2")), div((class("innerdiv"), div((Some(elements::span("test")), "nochmal text", br(()), "test after newline")), div("test")))).split_into_attributes_and_children());

    assert_eq!(
        div((
            class("DivClass"),
            id("DivId"),
            p("This is the first paragraph."),
            p((
                "Second paragraph contains a ",
                a((href("www.example.com"), "link")),
                " and",
                br(()),
                "a linebreak."
            )),
        )).to_string(),
        r#"<div class="DivClass" id="DivId"><p>This is the first paragraph.</p><p>Second paragraph contains a <a href="www.example.com">link</a> and<br/>a linebreak.</p></div>"#,
    );
}

#[derive(Debug, Clone)]
pub struct HtmlElement<AoC: AttributeOrChild> {
    tag_name: &'static str, // TODO impl Borrow<str>?
    attributes: AoC::Attribute,
    children: AoC::Child,
}

impl<AoC: AttributeOrChild> HtmlElement<AoC> {
    pub fn new(tag_name: &'static str, attributeorchild: AoC) -> Self {
        let (attributes, children) = attributeorchild.split_into_attributes_and_children();
        Self{tag_name, attributes, children}
    }
}

#[derive(Debug, Clone)]
pub struct VoidElement<A>
    where (A, ()): AttributeOrChild
{
    tag_name: &'static str, // TODO impl Borrow<str>?
    attributes: A,
}

impl<A> VoidElement<A>
    where (A, ()): AttributeOrChild
{
    pub fn new(tag_name: &'static str, attributes: A) -> Self {
        Self{tag_name, attributes}
    }
}

macro_rules! for_each_element{($m:ident) => {
    // Scraped from https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements, https://developer.mozilla.org/en-US/docs/Glossary/Void_element
    $m!(a);
    $m!(abbr);
    $m!(acronym);
    $m!(address);
    $m!(area, void_element);
    $m!(article);
    $m!(aside);
    $m!(audio);
    $m!(b);
    $m!(base, void_element);
    $m!(bdi);
    $m!(bdo);
    $m!(big);
    $m!(blockquote);
    $m!(body);
    $m!(br, void_element);
    $m!(button);
    $m!(canvas);
    $m!(caption);
    $m!(center);
    $m!(code);
    $m!(col, void_element);
    $m!(colgroup);
    $m!(datalist);
    $m!(dd);
    $m!(del);
    $m!(details);
    $m!(dfn);
    $m!(dialog);
    $m!(div);
    $m!(dl);
    $m!(dt);
    $m!(em);
    $m!(embed, void_element);
    $m!(fencedframe);
    $m!(fieldset);
    $m!(figcaption);
    $m!(figure);
    $m!(font);
    $m!(footer);
    $m!(frame);
    $m!(frameset);
    $m!(h1);
    $m!(h2);
    $m!(h3);
    $m!(h4);
    $m!(h5);
    $m!(h6);
    $m!(head);
    $m!(header);
    $m!(hgroup);
    $m!(hr, void_element);
    $m!(html);
    $m!(i);
    $m!(iframe);
    $m!(img, void_element);
    $m!(input, void_element);
    $m!(ins);
    $m!(kbd);
    $m!(legend);
    $m!(li);
    $m!(link, void_element);
    $m!(main);
    $m!(map);
    $m!(mark);
    $m!(marquee);
    $m!(menu);
    $m!(meta, void_element);
    $m!(meter);
    $m!(nav);
    $m!(nobr);
    $m!(noembed);
    $m!(noframes);
    $m!(noscript);
    $m!(object);
    $m!(ol);
    $m!(optgroup);
    $m!(option);
    $m!(output);
    $m!(p);
    $m!(param, void_element);
    $m!(picture);
    $m!(plaintext);
    $m!(pre);
    $m!(progress);
    $m!(q);
    $m!(rb);
    $m!(rp);
    $m!(rt);
    $m!(rtc);
    $m!(ruby);
    $m!(s);
    $m!(samp);
    $m!(script);
    $m!(search);
    $m!(section);
    $m!(select);
    $m!(selectedcontent);
    $m!(small);
    $m!(source, void_element);
    $m!(strike);
    $m!(strong);
    $m!(sub);
    $m!(sup);
    $m!(table);
    $m!(tbody);
    $m!(td);
    $m!(template);
    $m!(textarea);
    $m!(tfoot);
    $m!(th);
    $m!(thead);
    $m!(time);
    $m!(tr);
    $m!(track, void_element);
    $m!(tt);
    $m!(u);
    $m!(ul);
    $m!(var);
    $m!(video);
    $m!(wbr, void_element);
    $m!(xmp);
}}

macro_rules! for_each_attribute_and_element{($m:ident) => {
    $m!(cite);
    $m!(data);
    $m!(dir);
    $m!(form);
    $m!(label);
    $m!(slot);
    $m!(span);
    $m!(style);
    $m!(summary);
    $m!(title);
}}

pub mod elements {
    use super::*;
    macro_rules! impl_element(
        ($tag_name:ident) => {
            pub fn $tag_name<AoC: AttributeOrChild>(attributes_and_children: AoC) -> HtmlElement<AoC> {
                HtmlElement::new(stringify!($tag_name), attributes_and_children)
            }
        };
        ($tag_name:ident, void_element) => {
            pub fn $tag_name<A>(attributes: A) -> VoidElement<A>
                where
                    (A, ()): AttributeOrChild,
            {
                VoidElement::new(stringify!($tag_name), attributes)
            }
        };
    );

    for_each_element!(impl_element);
    for_each_attribute_and_element!(impl_element);
}
macro_rules! pub_use_element{
    ($tag_name:ident $(, void_element)?) => {
        pub use elements::$tag_name;
    };
    ($tag_name:ident, no_pub_use) => {};
}
for_each_element!(pub_use_element);

impl<AoC: AttributeOrChild> Display for HtmlElement<AoC> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt_child(formatter)
    }
}
impl<AoC: AttributeOrChild> Display for VoidElement<AoC> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt_child(formatter)
    }
}
pub fn html_display_children(t: impl AttributeOrChild) -> impl Display {
    struct SDisplay<T: AttributeOrChild>(T);
    impl<AoC: AttributeOrChild> Display for SDisplay<AoC> {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            self.0.fmt_child(formatter)
        }
    }
    SDisplay(t)
}

#[derive(Debug, Clone)]
pub struct SHtmlAttr<StrKey, StrVal>(StrKey, StrVal);

macro_rules! for_each_attribute{($m:ident) => {
    $m!(accept);
    $m!(accept_charset);
    $m!(accesskey);
    $m!(action);
    $m!(align);
    $m!(allow);
    $m!(alt);
    $m!(as_); // "as" is a Rust keyword
    $m!(async_);
    $m!(autocapitalize);
    $m!(autocomplete);
    $m!(autoplay);
    $m!(background);
    $m!(bgcolor);
    $m!(border);
    $m!(capture);
    $m!(charset);
    $m!(checked);
    $m!(class);
    $m!(color);
    $m!(cols);
    $m!(colspan);
    $m!(content);
    $m!(contenteditable);
    $m!(controls);
    $m!(coords);
    $m!(crossorigin);
    $m!(csp);
    $m!(data_star); // asterisk cannot be used
    $m!(datetime);
    $m!(decoding);
    $m!(default);
    $m!(defer);
    $m!(dirname);
    $m!(disabled);
    $m!(download);
    $m!(draggable);
    $m!(enctype);
    $m!(enterkeyhint);
    $m!(elementtiming);
    $m!(for_); // "for" is a Rust keyword
    $m!(formaction);
    $m!(formenctype);
    $m!(formmethod);
    $m!(formnovalidate);
    $m!(formtarget);
    $m!(headers);
    $m!(height);
    $m!(hidden);
    $m!(high);
    $m!(href);
    $m!(hreflang);
    $m!(http_equiv);
    $m!(id);
    $m!(integrity);
    $m!(inputmode);
    $m!(ismap);
    $m!(itemprop);
    $m!(kind);
    $m!(lang);
    $m!(language);
    $m!(loading);
    $m!(list);
    $m!(loop_); // "loop" is a Rust keyword
    $m!(low);
    $m!(max);
    $m!(maxlength);
    $m!(minlength);
    $m!(media);
    $m!(method);
    $m!(min);
    $m!(multiple);
    $m!(muted);
    $m!(name);
    $m!(novalidate);
    $m!(open);
    $m!(optimum);
    $m!(pattern);
    $m!(ping);
    $m!(placeholder);
    $m!(playsinline);
    $m!(poster);
    $m!(preload);
    $m!(readonly);
    $m!(referrerpolicy);
    $m!(rel);
    $m!(required);
    $m!(reversed);
    $m!(role);
    $m!(rows);
    $m!(rowspan);
    $m!(sandbox);
    $m!(scope);
    $m!(selected);
    $m!(shape);
    $m!(size);
    $m!(sizes);
    $m!(spellcheck);
    $m!(src);
    $m!(srcdoc);
    $m!(srclang);
    $m!(srcset);
    $m!(start);
    $m!(step);
    $m!(tabindex);
    $m!(target);
    $m!(translate);
    $m!(type_); // "type" is a Rust keyword
    $m!(usemap);
    $m!(value);
    $m!(width);
    $m!(wrap);
}}

pub mod attributes {
    use super::*;
    macro_rules! impl_attr(($attr:ident) => {
        pub fn $attr<StrVal: std::borrow::Borrow<str>>(str_val: StrVal) -> SHtmlAttr<&'static str, StrVal> {
            SHtmlAttr(stringify!($attr), str_val)
        }
    });
    for_each_attribute!(impl_attr);
    for_each_attribute_and_element!(impl_attr);
}
macro_rules! pub_use_attribute{($tag_name:ident) => {
    pub use attributes::$tag_name;
}}
for_each_attribute!(pub_use_attribute);

impl<AoC: AttributeOrChild> AttributeOrChild for Vec<AoC> {
    type Attribute = Vec<AoC::Attribute>;
    type Child = Vec<AoC::Child>;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        let mut vecattribute = Vec::new();
        let mut vecchild = Vec::new();
        for attributeorchild in self {
            let (attribute, child) = attributeorchild.split_into_attributes_and_children();
            vecattribute.push(attribute);
            vecchild.push(child);
        }
        (vecattribute, vecchild)
    }
    fn fmt_attr(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for attributeorchild in self {
            attributeorchild.fmt_attr(formatter)?;
        }
        Ok(())
    }
    fn fmt_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for attributeorchild in self {
            attributeorchild.fmt_child(formatter)?;
        }
        Ok(())
    }
}
impl<AoC: AttributeOrChild> AttributeOrChild for Option<AoC> {
    type Attribute = Option<AoC::Attribute>;
    type Child = Option<AoC::Child>;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        if let Some(attributeorchild) = self {
            let (attribute, child) = attributeorchild.split_into_attributes_and_children();
            (Some(attribute), Some(child))
        } else {
            (None, None)
        }
    }
    fn fmt_attr(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Some(attributeorchild) = self {
            attributeorchild.fmt_attr(formatter)?;
        }
        Ok(())
    }
    fn fmt_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Some(attributeorchild) = self {
            attributeorchild.fmt_child(formatter)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct HtmlIter<Iter>(Iter);

impl<Iter: Iterator + Clone> AttributeOrChild for HtmlIter<Iter>
    where
        Iter::Item: AttributeOrChild,
{
    type Attribute = Self;
    type Child = Self;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        (self.clone(), self)
    }
    fn fmt_attr(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for attributeorchild in self.0.clone() {
            attributeorchild.fmt_attr(formatter)?;
        }
        Ok(())
    }
    fn fmt_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for attributeorchild in self.0.clone() {
            attributeorchild.fmt_child(formatter)?;
        }
        Ok(())
    }
}

pub fn html_iter<Iter: Iterator + Clone>(it: Iter) -> HtmlIter<Iter>
    where
        Iter::Item: AttributeOrChild,
{
    HtmlIter(it)
}
