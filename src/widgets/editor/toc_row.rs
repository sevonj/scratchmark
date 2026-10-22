mod imp {
    use crate::data::MarkdownHeading;
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::Label;
    use gtk::ListBoxRow;
    use gtk::TemplateChild;
    use gtk::glib;
    use gtk::glib::Properties;
    use std::cell::RefCell;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::TocRow)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/editor/toc_row.ui")]
    pub struct TocRow {
        #[template_child]
        pub(super) label: TemplateChild<Label>,
        #[template_child]
        pub(super) line: TemplateChild<Label>,

        pub(super) heading: RefCell<Option<MarkdownHeading>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TocRow {
        const NAME: &'static str = "TocRow";
        type Type = super::TocRow;
        type ParentType = ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TocRow {}

    impl WidgetImpl for TocRow {}
    impl ListBoxRowImpl for TocRow {}

    impl TocRow {
        pub(super) fn update(&self, heading: MarkdownHeading) {
            let heading_text = heading.text();
            let line_str = (heading.line() + 1).to_string();

            if self.label.text() != heading_text {
                self.label.set_text(heading_text);
            }
            if self.line.text() != line_str {
                self.line.set_text(&line_str);
            }
            self.heading.replace(Some(heading));
        }
    }
}

use crate::data::MarkdownHeading;
use adw::subclass::prelude::*;
use gtk::ListBoxRow;
use gtk::glib;
use gtk::glib::Object;
use std::cell::Ref;

glib::wrapper! {
    pub struct TocRow(ObjectSubclass<imp::TocRow>)
        @extends ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl TocRow {
    pub fn new(heading: MarkdownHeading) -> Self {
        let obj: Self = Object::builder().build();
        obj.update(heading);
        obj
    }

    pub fn update(&self, heading: MarkdownHeading) {
        self.imp().update(heading);
    }

    pub fn heading(&self) -> Ref<'_, MarkdownHeading> {
        Ref::map(self.imp().heading.borrow(), |opt| {
            opt.as_ref()
                .expect("TocRow: heading is unset, this should be impossible.")
        })
    }
}
