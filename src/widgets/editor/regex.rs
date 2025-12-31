use std::sync::LazyLock;

use regex::Regex;

/*
 * If you touch the regex here, make sure to do the same to the lang spec!
 */

pub static ATX_H_OPENING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^ {0,3}#{1,6}($| +)").unwrap());

#[cfg(test)]
mod tests {
    use crate::widgets::editor::regex::ATX_H_OPENING;

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
