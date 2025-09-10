mod imp {
    use adw::subclass::prelude::*;
    use gtk::glib;

    use gtk::CompositeTemplate;
    use gtk::Label;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/sheet_stats.ui")]
    pub struct SheetStats {
        #[template_child]
        pub(super) lab_num_chars: TemplateChild<Label>,
        #[template_child]
        pub(super) lab_num_nospace: TemplateChild<Label>,
        #[template_child]
        pub(super) lab_num_words: TemplateChild<Label>,
        #[template_child]
        pub(super) lab_num_lines: TemplateChild<Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SheetStats {
        const NAME: &'static str = "SheetStats";
        type Type = super::SheetStats;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SheetStats {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for SheetStats {}
    impl BinImpl for SheetStats {}
}

use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::glib;

use crate::widgets::sheet_editor::SheetStatsData;

glib::wrapper! {
    pub struct SheetStats(ObjectSubclass<imp::SheetStats>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for SheetStats {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl SheetStats {
    pub fn set_stats(&self, data: &SheetStatsData) {
        let imp = self.imp();
        imp.lab_num_chars.set_label(&format!("{}", data.num_chars));
        imp.lab_num_nospace
            .set_label(&format!("{}", data.num_chars - data.num_spaces));
        imp.lab_num_words.set_label(&format!("{}", data.num_words));
        imp.lab_num_lines.set_label(&format!("{}", data.num_lines));
    }
}
