mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use gtk::glib;
    use gtk::prelude::*;
    use sourceview5::prelude::*;

    use gtk::Button;
    use gtk::CompositeTemplate;
    use gtk::Entry;
    use gtk::Label;
    use gtk::SearchBar;
    use gtk::TextIter;
    use gtk::TextMark;
    use gtk::ToggleButton;
    use gtk::gio::Cancellable;
    use gtk::gio::SimpleAction;
    use gtk::gio::SimpleActionGroup;
    use gtk::glib::VariantTy;
    use gtk::glib::clone;
    use gtk::glib::subclass::Signal;
    use sourceview5::SearchContext;
    use sourceview5::SearchSettings;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/editor/search_bar.ui")]
    pub struct EditorSearchBar {
        actions: SimpleActionGroup,

        #[template_child]
        search_bar: TemplateChild<SearchBar>,
        #[template_child]
        search_entry: TemplateChild<Entry>,
        #[template_child]
        search_match_case_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        search_match_whole_words_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        search_match_regex_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        search_replace_entry: TemplateChild<Entry>,
        #[template_child]
        search_occurrences_label: TemplateChild<Label>,
        #[template_child]
        search_prev_button: TemplateChild<Button>,
        #[template_child]
        search_next_button: TemplateChild<Button>,
        #[template_child]
        search_replace_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        search_replace_button: TemplateChild<Button>,
        #[template_child]
        search_replace_all_button: TemplateChild<Button>,
        #[template_child]
        search_replace_buttons_container: TemplateChild<gtk::Box>,

        search_settings: RefCell<Option<SearchSettings>>,
        search_context: RefCell<Option<SearchContext>>,
        search_position: Cell<Option<i32>>,
        search_occurrences: Cell<Option<i32>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EditorSearchBar {
        const NAME: &'static str = "EditorSearchBar";
        type Type = super::EditorSearchBar;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EditorSearchBar {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            self.setup_actions();

            self.search_entry.connect_changed(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.refresh();
                }
            ));
            self.search_entry.connect_activate(clone!(
                #[weak]
                obj,
                move |_: &Entry| {
                    obj.activate_action("search.search-next", None).unwrap();
                }
            ));
            self.search_replace_entry.connect_activate(clone!(
                #[weak]
                obj,
                move |_: &Entry| {
                    obj.activate_action("search.commit-replace", None).unwrap();
                }
            ));

            let search_replace_toggle: &ToggleButton = self.search_replace_toggle.as_ref();
            self.search_replace_buttons_container
                .bind_property("visible", search_replace_toggle, "active")
                .bidirectional()
                .sync_create()
                .build();
            self.search_replace_entry
                .bind_property("visible", search_replace_toggle, "active")
                .bidirectional()
                .sync_create()
                .build();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("scroll-to-mark")
                        .param_types([TextMark::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for EditorSearchBar {}
    impl BinImpl for EditorSearchBar {}

    impl EditorSearchBar {
        fn refresh(&self) {
            self.search_settings
                .borrow()
                .as_ref()
                .unwrap()
                .set_search_text(Some(&self.search_entry.text()));
        }

        fn setup_actions(&self) {
            let obj = self.obj();

            obj.insert_action_group("search", Some(&self.actions));

            let action = SimpleAction::new("search", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _| {
                    imp.search_bar.set_search_mode(true);
                    imp.search_entry.grab_focus();
                    imp.search_context
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .set_highlight(true);
                    imp.search_replace_buttons_container.set_visible(false);
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("search-with-text", Some(VariantTy::STRING));
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, text| {
                    imp.obj().activate_action("search.search", None).unwrap();
                    let text: String = text.unwrap().get().unwrap();
                    imp.search_entry.set_text(&text);
                    imp.refresh();
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("search-replace", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _| {
                    imp.search_bar.set_search_mode(true);
                    imp.search_replace_entry.grab_focus();
                    imp.search_context
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .set_highlight(true);
                    imp.search_replace_buttons_container.set_visible(true);
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("hide", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _| {
                    imp.search_bar.set_search_mode(false);
                    imp.search_context
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .set_highlight(false);
                    imp.search_replace_buttons_container.set_visible(false);
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("search-prev", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _| {
                    if imp.search_occurrences.get().unwrap_or(0) < 1 {
                        return;
                    }
                    let search_context_bind = imp.search_context.borrow();
                    let search_context = search_context_bind.as_ref().unwrap();
                    let mark = search_context.buffer().get_insert();
                    let iter = search_context.buffer().iter_at_mark(&mark);
                    search_context.backward_async(
                        &iter,
                        None::<&Cancellable>,
                        clone!(
                            #[weak]
                            imp,
                            move |result| {
                                match result {
                                    Ok((start, end, _wrapped)) => {
                                        imp.update_search_position(Some((start, end)))
                                    }
                                    Err(_) => imp.update_search_position(None),
                                }
                            }
                        ),
                    );
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("search-next", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _| {
                    if imp.search_occurrences.get().unwrap_or(0) < 1 {
                        return;
                    }
                    let search_context_bind = imp.search_context.borrow();
                    let search_context = search_context_bind.as_ref().unwrap();
                    let mark = search_context.buffer().selection_bound();
                    let iter = search_context.buffer().iter_at_mark(&mark);
                    search_context.forward_async(
                        &iter,
                        None::<&Cancellable>,
                        clone!(
                            #[weak]
                            imp,
                            move |result| {
                                match result {
                                    Ok((start, end, _wrapped)) => {
                                        imp.update_search_position(Some((start, end)))
                                    }
                                    Err(_) => imp.update_search_position(None),
                                }
                            }
                        ),
                    );
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("commit-replace", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _| {
                    if imp.search_occurrences.get().unwrap_or(0) < 1 {
                        return;
                    }
                    let search_context_bind = imp.search_context.borrow();
                    let search_context = search_context_bind.as_ref().unwrap();
                    let mark = search_context.buffer().get_insert();
                    let iter = search_context.buffer().iter_at_mark(&mark);
                    let text = imp.search_replace_entry.text();

                    search_context.forward_async(
                        &iter,
                        None::<&Cancellable>,
                        clone!(
                            #[weak]
                            imp,
                            move |result| {
                                let search_context_bind = imp.search_context.borrow();
                                let search_context = search_context_bind.as_ref().unwrap();
                                match result {
                                    Ok((mut match_start, mut match_end, _wrapped)) => {
                                        let _ = search_context.replace(
                                            &mut match_start,
                                            &mut match_end,
                                            &text,
                                        );
                                    }
                                    Err(_) => imp.update_search_position(None),
                                }
                            }
                        ),
                    );
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("commit-replace-all", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _| {
                    let search_context_bind = imp.search_context.borrow();
                    let search_context = search_context_bind.as_ref().unwrap();
                    let text = imp.search_replace_entry.text();
                    let _ = search_context.replace_all(&text);
                }
            ));
            self.actions.add_action(&action);

            // This action is a workaround to capture <Shift>Return from the Entry
            let action = SimpleAction::new("shiftreturn", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _| {
                    let Some(currently_focused) = imp.obj().root().and_then(|r| r.focus()) else {
                        return;
                    };
                    let search_entry: &Entry = imp.search_entry.as_ref();
                    let replace_entry: &Entry = imp.search_replace_entry.as_ref();
                    if currently_focused.is_ancestor(search_entry) {
                        imp.obj()
                            .activate_action("search.search-prev", None)
                            .unwrap();
                    } else if currently_focused.is_ancestor(replace_entry) {
                        imp.obj()
                            .activate_action("search.commit-replace-all", None)
                            .unwrap();
                    }
                }
            ));
            self.actions.add_action(&action);
        }

        pub(super) fn is_search_focused(&self) -> bool {
            let Some(currently_focused) = self.obj().root().and_then(|r| r.focus()) else {
                return false;
            };
            let search_entry: &Entry = self.search_entry.as_ref();
            let replace_entry: &Entry = self.search_replace_entry.as_ref();
            currently_focused.is_ancestor(search_entry)
                || currently_focused.is_ancestor(replace_entry)
        }

        pub(super) fn set_search_context(&self, search_context: SearchContext) {
            search_context.connect_occurrences_count_notify(clone!(
                #[weak(rename_to = imp)]
                self,
                move |search_context: &SearchContext| {
                    let cnt = search_context.occurrences_count();
                    imp.search_occurrences.replace(Some(cnt));

                    let found_any = cnt > 0;
                    imp.search_prev_button.set_sensitive(found_any); // TODO: Disable action instead
                    imp.search_next_button.set_sensitive(found_any); // TODO: Disable action instead
                    imp.search_replace_all_button.set_sensitive(found_any); // TODO: Disable action instead
                    if !found_any {
                        imp.update_search_position(None);
                        imp.update_search_occurrence_text();
                        return;
                    }

                    if imp.is_search_focused() {
                        let mark = search_context.buffer().get_insert();
                        let iter = search_context.buffer().iter_at_mark(&mark);
                        search_context.forward_async(
                            &iter,
                            None::<&Cancellable>,
                            clone!(
                                #[weak]
                                imp,
                                move |result| {
                                    match result {
                                        Ok((start, end, _wrapped)) => {
                                            imp.update_search_position(Some((start, end)))
                                        }
                                        Err(_) => imp.update_search_position(None),
                                    }
                                }
                            ),
                        );
                    } else {
                        imp.update_search_position(None);
                    }
                    imp.update_search_occurrence_text();
                }
            ));
            let search_settings = search_context.settings();
            self.search_match_case_toggle
                .bind_property("active", &search_settings, "case-sensitive")
                .bidirectional()
                .sync_create()
                .build();
            self.search_match_whole_words_toggle
                .bind_property("active", &search_settings, "at-word-boundaries")
                .bidirectional()
                .sync_create()
                .build();
            self.search_match_regex_toggle
                .bind_property("active", &search_settings, "regex-enabled")
                .bidirectional()
                .sync_create()
                .build();
            self.search_context.replace(Some(search_context));
            self.search_settings.replace(Some(search_settings));
        }

        fn update_search_position(&self, result: Option<(TextIter, TextIter)>) {
            let Some((match_start, match_end)) = result else {
                self.search_position.replace(None);
                self.update_search_occurrence_text();
                self.search_replace_button.set_sensitive(false);
                return;
            };

            let search_context_bind = self.search_context.borrow();
            let search_context = search_context_bind.as_ref().unwrap();
            let pos = search_context.occurrence_position(&match_start, &match_end);
            self.search_replace_button.set_sensitive(pos >= 1);
            self.search_position.replace(Some(pos));
            self.update_search_occurrence_text();

            search_context
                .buffer()
                .select_range(&match_start, &match_end);

            let mark = search_context.buffer().get_insert();
            self.obj().emit_by_name::<()>("scroll-to-mark", &[&mark]);
        }

        fn update_search_occurrence_text(&self) {
            let pos = match self.search_position.get() {
                Some(value) if value >= 1 => value.to_string(),
                _ => "?".into(),
            };
            let cnt = match self.search_occurrences.get() {
                Some(value) if value >= 0 => value.to_string(),
                _ => "?".into(),
            };
            self.search_occurrences_label
                .set_text(&format!("{pos} of {cnt}"));
        }
    }
}

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use sourceview5::SearchContext;

glib::wrapper! {
    pub struct EditorSearchBar(ObjectSubclass<imp::EditorSearchBar>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for EditorSearchBar {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl EditorSearchBar {
    pub fn set_search_context(&self, search_context: SearchContext) {
        self.imp().set_search_context(search_context);
    }
}
