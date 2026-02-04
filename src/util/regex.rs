use std::sync::LazyLock;

use regex::Regex;

/*
 * Scroll down for langspec regex tests
 * If you touch the regex here, make sure to do the same to the langspec!
 */

pub static ATX_H_OPENING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^ {0,3}#{1,6}($| +)").unwrap());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atx_h_opening_levels() {
        let h1 = "# foo";
        let h2 = "## foo";
        let h3 = "### foo";
        let h4 = "#### foo";
        let h5 = "##### foo";
        let h6 = "###### foo";

        assert_eq!(ATX_H_OPENING.find(h1).unwrap().as_str(), "# ");
        assert_eq!(ATX_H_OPENING.find(h2).unwrap().as_str(), "## ");
        assert_eq!(ATX_H_OPENING.find(h3).unwrap().as_str(), "### ");
        assert_eq!(ATX_H_OPENING.find(h4).unwrap().as_str(), "#### ");
        assert_eq!(ATX_H_OPENING.find(h5).unwrap().as_str(), "##### ");
        assert_eq!(ATX_H_OPENING.find(h6).unwrap().as_str(), "###### ");
    }

    #[test]
    fn test_atx_h_opening_level_too_high() {
        let h7 = "####### foo";
        assert!(ATX_H_OPENING.find(h7).is_none());
    }

    #[test]
    fn test_atx_h_opening_no_space() {
        let nospace_h1 = "#foo";
        let nospace_h2 = "##foo";
        assert!(ATX_H_OPENING.find(nospace_h1).is_none());
        assert!(ATX_H_OPENING.find(nospace_h2).is_none());
    }

    #[test]
    fn test_atx_h_opening_escaped() {
        let escaped_h1 = "\\# foo";
        let escaped_h2 = "\\## foo";
        assert!(ATX_H_OPENING.find(escaped_h1).is_none());
        assert!(ATX_H_OPENING.find(escaped_h2).is_none());
    }

    #[test]
    fn test_atx_h_opening_indent() {
        let h1a = " # foo";
        let h2a = " ## foo";
        let h3a = " ### foo";
        let h4a = " #### foo";
        let h5a = " ##### foo";
        let h6a = " ###### foo";
        let h1b = "  # foo";
        let h2b = "  ## foo";
        let h3b = "  ### foo";
        let h4b = "  #### foo";
        let h5b = "  ##### foo";
        let h6b = "  ###### foo";
        let h1c = "   # foo";
        let h2c = "   ## foo";
        let h3c = "   ### foo";
        let h4c = "   #### foo";
        let h5c = "   ##### foo";
        let h6c = "   ###### foo";
        let h1d = "    # foo";
        let h2d = "    ## foo";
        let h3d = "    ### foo";
        let h4d = "    #### foo";
        let h5d = "    ##### foo";
        let h6d = "    ###### foo";

        assert_eq!(ATX_H_OPENING.find(h1a).unwrap().as_str(), " # ");
        assert_eq!(ATX_H_OPENING.find(h2a).unwrap().as_str(), " ## ");
        assert_eq!(ATX_H_OPENING.find(h3a).unwrap().as_str(), " ### ");
        assert_eq!(ATX_H_OPENING.find(h4a).unwrap().as_str(), " #### ");
        assert_eq!(ATX_H_OPENING.find(h5a).unwrap().as_str(), " ##### ");
        assert_eq!(ATX_H_OPENING.find(h6a).unwrap().as_str(), " ###### ");
        assert_eq!(ATX_H_OPENING.find(h1b).unwrap().as_str(), "  # ");
        assert_eq!(ATX_H_OPENING.find(h2b).unwrap().as_str(), "  ## ");
        assert_eq!(ATX_H_OPENING.find(h3b).unwrap().as_str(), "  ### ");
        assert_eq!(ATX_H_OPENING.find(h4b).unwrap().as_str(), "  #### ");
        assert_eq!(ATX_H_OPENING.find(h5b).unwrap().as_str(), "  ##### ");
        assert_eq!(ATX_H_OPENING.find(h6b).unwrap().as_str(), "  ###### ");
        assert_eq!(ATX_H_OPENING.find(h1c).unwrap().as_str(), "   # ");
        assert_eq!(ATX_H_OPENING.find(h2c).unwrap().as_str(), "   ## ");
        assert_eq!(ATX_H_OPENING.find(h3c).unwrap().as_str(), "   ### ");
        assert_eq!(ATX_H_OPENING.find(h4c).unwrap().as_str(), "   #### ");
        assert_eq!(ATX_H_OPENING.find(h5c).unwrap().as_str(), "   ##### ");
        assert_eq!(ATX_H_OPENING.find(h6c).unwrap().as_str(), "   ###### ");
        assert!(ATX_H_OPENING.find(h1d).is_none());
        assert!(ATX_H_OPENING.find(h2d).is_none());
        assert!(ATX_H_OPENING.find(h3d).is_none());
        assert!(ATX_H_OPENING.find(h4d).is_none());
        assert!(ATX_H_OPENING.find(h5d).is_none());
        assert!(ATX_H_OPENING.find(h6d).is_none());
    }

    #[test]
    fn test_atx_h_opening_can_be_empty_nospace() {
        let empty_h1 = "#";
        let empty_h2 = "##";
        let empty_h3 = "###";
        assert_eq!(ATX_H_OPENING.find(empty_h1).unwrap().as_str(), "#");
        assert_eq!(ATX_H_OPENING.find(empty_h2).unwrap().as_str(), "##");
        assert_eq!(ATX_H_OPENING.find(empty_h3).unwrap().as_str(), "###");
    }

    #[test]
    fn test_atx_h_opening_can_be_empty_withspace() {
        let empty_h1 = "# ";
        let empty_h2 = "## ";
        let empty_h3 = "### ";
        assert_eq!(ATX_H_OPENING.find(empty_h1).unwrap().as_str(), "# ");
        assert_eq!(ATX_H_OPENING.find(empty_h2).unwrap().as_str(), "## ");
        assert_eq!(ATX_H_OPENING.find(empty_h3).unwrap().as_str(), "### ");
    }
}

#[cfg(test)]
mod langspec_tests {
    use std::sync::LazyLock;

    use regex::Regex;

    static THEMATIC_BREAK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^ {0,3}((-( |\t)*){3,}|(_( |\t)*){3,}|(\*( |\t)*){3,})$").unwrap()
    });

    static ATX_H1_WHOLELINE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^ {0,3}#($| +.*)").unwrap());
    static ATX_H2_WHOLELINE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^ {0,3}##($| +.*)").unwrap());
    static ATX_H3_WHOLELINE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^ {0,3}###($| +.*)").unwrap());
    static ATX_H4_WHOLELINE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^ {0,3}####($| +.*)").unwrap());
    static ATX_H5_WHOLELINE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^ {0,3}#####($| +.*)").unwrap());
    static ATX_H6_WHOLELINE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^ {0,3}######($| +.*)").unwrap());

    fn heading_test_suite(regex: &LazyLock<Regex>, heading_level: usize) {
        let h = String::from("#").repeat(heading_level as usize);

        let h1 = format!("{h} foo");
        assert_eq!(regex.find(&h1).unwrap().as_str(), h1);

        let h1_ind1 = format!(" {h} foo");
        let h1_ind2 = format!("  {h} foo");
        let h1_ind3 = format!("   {h} foo");
        let h1_ind4 = format!("    {h} foo");
        assert_eq!(regex.find(&h1_ind1).unwrap().as_str(), h1_ind1);
        assert_eq!(regex.find(&h1_ind2).unwrap().as_str(), h1_ind2);
        assert_eq!(regex.find(&h1_ind3).unwrap().as_str(), h1_ind3);
        assert!(regex.find(&h1_ind4).is_none());

        let h1_closing1 = format!("{h} foo {h}");
        let h1_closing2 = format!("{h} foo #");
        let h1_closing3 = format!("{h} foo ###");
        let h1_closing4 = format!("{h} foo ##  # ##");
        let h1_closing5 = format!("{h} foo ## b");
        assert_eq!(regex.find(&h1_closing1).unwrap().as_str(), h1_closing1);
        assert_eq!(regex.find(&h1_closing2).unwrap().as_str(), h1_closing2);
        assert_eq!(regex.find(&h1_closing3).unwrap().as_str(), h1_closing3);
        assert_eq!(regex.find(&h1_closing4).unwrap().as_str(), h1_closing4);
        assert_eq!(regex.find(&h1_closing5).unwrap().as_str(), h1_closing5);

        let h1_empty1 = format!("{h}");
        let h1_empty2 = format!("{h} ");
        assert_eq!(regex.find(&h1_empty1).unwrap().as_str(), h1_empty1);
        assert_eq!(regex.find(&h1_empty2).unwrap().as_str(), h1_empty2);

        // Check for false positives from different size headings
        for i in 1..=7 {
            if i == heading_level {
                continue;
            }
            let hi = format!("{} foo", String::from("#").repeat(i));
            assert!(regex.find(&hi).is_none());
        }
    }

    #[test]
    fn test_thematic_break() {
        let notenuf_a = "--";
        let notenuf_b = "__";
        let notenuf_c = "**";
        assert!(THEMATIC_BREAK.find(&notenuf_a).is_none());
        assert!(THEMATIC_BREAK.find(&notenuf_b).is_none());
        assert!(THEMATIC_BREAK.find(&notenuf_c).is_none());
        let mismatch = "*-*";
        assert!(THEMATIC_BREAK.find(&mismatch).is_none());
        let plain_a = "---";
        let plain_b = "___";
        let plain_c = "***";
        assert_eq!(THEMATIC_BREAK.find(&plain_a).unwrap().as_str(), "---");
        assert_eq!(THEMATIC_BREAK.find(&plain_b).unwrap().as_str(), "___");
        assert_eq!(THEMATIC_BREAK.find(&plain_c).unwrap().as_str(), "***");
        let long_a = "----";
        let long_b = "__________";
        let long_c =
            "*******************************************************************************";
        assert_eq!(THEMATIC_BREAK.find(&long_a).unwrap().as_str(), "----");
        assert_eq!(THEMATIC_BREAK.find(&long_b).unwrap().as_str(), "__________");
        assert_eq!(
            THEMATIC_BREAK.find(&long_c).unwrap().as_str(),
            "*******************************************************************************"
        );
        let plain_line_a = "\n---";
        let plain_line_b = "\n___";
        let plain_line_c = "\n***";
        assert_eq!(THEMATIC_BREAK.find(&plain_line_a).unwrap().as_str(), "---");
        assert_eq!(THEMATIC_BREAK.find(&plain_line_b).unwrap().as_str(), "___");
        assert_eq!(THEMATIC_BREAK.find(&plain_line_c).unwrap().as_str(), "***");
        let plain_line2_a = "---\n";
        let plain_line2_b = "___\n";
        let plain_line2_c = "***\n";
        assert_eq!(THEMATIC_BREAK.find(&plain_line2_a).unwrap().as_str(), "---");
        assert_eq!(THEMATIC_BREAK.find(&plain_line2_b).unwrap().as_str(), "___");
        assert_eq!(THEMATIC_BREAK.find(&plain_line2_c).unwrap().as_str(), "***");
        let plain_line2_a = "---\n";
        let plain_line2_b = "___\n";
        let plain_line2_c = "***\n";
        assert_eq!(THEMATIC_BREAK.find(&plain_line2_a).unwrap().as_str(), "---");
        assert_eq!(THEMATIC_BREAK.find(&plain_line2_b).unwrap().as_str(), "___");
        assert_eq!(THEMATIC_BREAK.find(&plain_line2_c).unwrap().as_str(), "***");
        let leading1 = " ---";
        let leading2 = "  ___";
        let leading3 = "   ***";
        let leading4 = "    ***";
        assert_eq!(THEMATIC_BREAK.find(&leading1).unwrap().as_str(), " ---");
        assert_eq!(THEMATIC_BREAK.find(&leading2).unwrap().as_str(), "  ___");
        assert_eq!(THEMATIC_BREAK.find(&leading3).unwrap().as_str(), "   ***");
        assert!(THEMATIC_BREAK.find(&leading4).is_none());
        let trailing1 = "---          ";
        let trailing2 = "aaabbbccc\n ___      \t\t\t        \naaabbbccc";
        let trailing3 = "  *** ";
        assert_eq!(
            THEMATIC_BREAK.find(&trailing1).unwrap().as_str(),
            "---          "
        );
        assert_eq!(
            THEMATIC_BREAK.find(&trailing2).unwrap().as_str(),
            " ___      \t\t\t        "
        );
        assert_eq!(THEMATIC_BREAK.find(&trailing3).unwrap().as_str(), "  *** ");
        let spaceinbetween_a = "- - -";
        let spaceinbetween_b = "_    _\t\t_";
        let spaceinbetween_c = "  *\t *\t*\t ";
        assert_eq!(
            THEMATIC_BREAK.find(&spaceinbetween_a).unwrap().as_str(),
            "- - -"
        );
        assert_eq!(
            THEMATIC_BREAK.find(&spaceinbetween_b).unwrap().as_str(),
            "_    _\t\t_"
        );
        assert_eq!(
            THEMATIC_BREAK.find(&spaceinbetween_c).unwrap().as_str(),
            "  *\t *\t*\t "
        );
    }

    #[test]
    fn test_atx_h1() {
        heading_test_suite(&ATX_H1_WHOLELINE, 1);
    }

    #[test]
    fn test_atx_h2() {
        heading_test_suite(&ATX_H2_WHOLELINE, 2);
    }

    #[test]
    fn test_atx_h3() {
        heading_test_suite(&ATX_H3_WHOLELINE, 3);
    }

    #[test]
    fn test_atx_h4() {
        heading_test_suite(&ATX_H4_WHOLELINE, 4);
    }

    #[test]
    fn test_atx_h5() {
        heading_test_suite(&ATX_H5_WHOLELINE, 5);
    }

    #[test]
    fn test_atx_h6() {
        heading_test_suite(&ATX_H6_WHOLELINE, 6);
    }
}
