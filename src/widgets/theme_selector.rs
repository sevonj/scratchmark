mod imp {
    use adw::ColorScheme;
    use adw::StyleManager;
    use adw::ToggleGroup;
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::glib;
    use gtk::glib::Properties;
    use gtk::prelude::*;
    use std::cell::RefCell;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::ThemeSelector)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/theme_selector.ui")]
    pub struct ThemeSelector {
        #[property(get, set)]
        theme: RefCell<String>,

        #[template_child]
        toggle_group: TemplateChild<ToggleGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ThemeSelector {
        const NAME: &'static str = "ThemeSelector";
        type Type = super::ThemeSelector;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_property_action("theme", "theme");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ThemeSelector {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let style_manager = StyleManager::default();

            let initial_theme = match style_manager.color_scheme() {
                ColorScheme::ForceLight | ColorScheme::PreferLight => "light",
                ColorScheme::ForceDark | ColorScheme::PreferDark => "dark",
                _ => "system",
            };
            obj.set_theme(initial_theme);

            let toggle_group: &ToggleGroup = &self.toggle_group;
            obj.bind_property("theme", toggle_group, "active-name")
                .bidirectional()
                .sync_create()
                .build();

            obj.connect_theme_notify(move |obj| {
                let theme = obj.theme();
                let style_manager = StyleManager::default();
                let target_scheme = match theme.as_str() {
                    "light" => ColorScheme::ForceLight,
                    "dark" => ColorScheme::ForceDark,
                    _ => ColorScheme::Default,
                };
                if style_manager.color_scheme() != target_scheme {
                    style_manager.set_color_scheme(target_scheme);
                }
            });

            style_manager.connect_color_scheme_notify(glib::clone!(
                #[weak]
                obj,
                move |sm| {
                    let current_theme = match sm.color_scheme() {
                        ColorScheme::ForceLight | ColorScheme::PreferLight => "light",
                        ColorScheme::ForceDark | ColorScheme::PreferDark => "dark",
                        _ => "system",
                    };
                    if obj.theme() != current_theme {
                        obj.set_theme(current_theme);
                    }
                }
            ));
        }
    }

    impl WidgetImpl for ThemeSelector {}
    impl BinImpl for ThemeSelector {}

    impl ThemeSelector {}
}

use gtk::glib;
use gtk::glib::Object;

glib::wrapper! {
    pub struct ThemeSelector(ObjectSubclass<imp::ThemeSelector>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ThemeSelector {
    fn default() -> Self {
        Object::builder().build()
    }
}
