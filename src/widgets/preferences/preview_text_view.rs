mod imp {
    use crate::data::MarkdownBuffer;
    use adw::StyleManager;
    use adw::subclass::prelude::*;
    use gtk::CssProvider;
    use gtk::glib;
    use gtk::glib::Properties;
    use gtk::prelude::*;
    use sourceview5::subclass::prelude::*;

    const SAMPLE_TEXT: &str = "\
    # Scratchmark\n\
    ## Scratchmark\n\
    ### Scratchmark\n\
    #### Scratchmark\n\
    ##### Scratchmark\n\
    ###### Scratchmark\n\
    ";

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::PreviewTextView)]
    pub struct PreviewTextView {
        pub(super) source_view_css_provider: CssProvider,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreviewTextView {
        const NAME: &'static str = "PreviewTextView";
        type Type = super::PreviewTextView;
        type ParentType = sourceview5::View;

        fn class_init(_klass: &mut Self::Class) {}

        fn instance_init(_obj: &glib::subclass::InitializingObject<Self>) {}
    }

    #[glib::derived_properties]
    impl ObjectImpl for PreviewTextView {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            // Deprecated, but the only way to do this at the moment?
            // https://gnome.pages.gitlab.gnome.org/gtksourceview/gtksourceview5/class.View.html#changing-the-font
            #[allow(deprecated)]
            obj.style_context().add_provider(
                &self.source_view_css_provider,
                gtk::ffi::GTK_STYLE_PROVIDER_PRIORITY_USER as u32,
            );

            let buf = MarkdownBuffer::default();
            buf.set_text(SAMPLE_TEXT);
            obj.set_buffer(Some(&buf));

            let sm = StyleManager::default();
            sm.connect_color_scheme_notify(glib::clone!(
                #[weak]
                obj,
                #[weak]
                buf,
                move |sm| {
                    buf.set_theme(if sm.is_dark() {
                        "scratchmark_dark"
                    } else {
                        "scratchmark_light"
                    });
                }
            ));
            buf.set_theme(if sm.is_dark() {
                "scratchmark_dark"
            } else {
                "scratchmark_light"
            });

            obj.set_editable(false);
        }
    }

    impl WidgetImpl for PreviewTextView {}
    impl TextViewImpl for PreviewTextView {}

    impl ViewImpl for PreviewTextView {}

    impl PreviewTextView {}
}

use crate::data::MarkdownBuffer;
use gtk::glib;
use gtk::glib::Object;
use gtk::subclass::prelude::*;
use sourceview5::prelude::*;

glib::wrapper! {
    pub struct PreviewTextView(ObjectSubclass<imp::PreviewTextView>)
        @extends sourceview5::View, gtk::TextView, gtk::Widget,
        @implements gtk::Accessible, gtk::AccessibleText, gtk::Buildable, gtk::ConstraintTarget, gtk::Scrollable;
}

impl Default for PreviewTextView {
    fn default() -> Self {
        println!("default");
        let obj: PreviewTextView = Object::builder().build();

        obj
    }
}

impl PreviewTextView {
    pub fn set_font(&self, family: &str, size: u32) {
        let formatted = format!("textview {{font-family: {family}; font-size: {size}pt;}}");
        self.imp()
            .source_view_css_provider
            .load_from_string(&formatted);
    }
}
