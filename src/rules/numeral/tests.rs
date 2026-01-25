use crate::rules::numeral;
use crate::{Context, Dimension, Options, TokenKind};

#[test]
fn numeral_examples_matching() {
    // Array of (expected_value, input_string)
    let cases: Vec<(f64, &str)> = vec![
        (0.0, "0"),
        (0.0, "naught"),
        (0.0, "nought"),
        (0.0, "zero"),
        (0.0, "nil"),
        (1.0, "1"),
        (1.0, "one"),
        (1.0, "single"),
        (2.0, "2"),
        (2.0, "two"),
        (2.0, "a pair"),
        (2.0, "a couple"),
        (2.0, "a couple of"),
        (3.0, "3"),
        (3.0, "three"),
        (3.0, "a few"),
        (3.0, "few"),
        (10.0, "10"),
        (10.0, "ten"),
        (12.0, "12"),
        (12.0, "twelve"),
        (12.0, "a dozen"),
        (12.0, "a dozen of"),
        (14.0, "14"),
        (14.0, "fourteen"),
        (16.0, "16"),
        (16.0, "sixteen"),
        (17.0, "17"),
        (17.0, "seventeen"),
        (18.0, "18"),
        (18.0, "eighteen"),
        (33.0, "33"),
        (33.0, "thirty three"),
        (33.0, "0033"),
        (1.1, "1.1"),
        (1.1, "1 point 1"),
        (0.77, ".77"),
        (0.77, "point 77"),
        (23.0, "twenty and three"),
        (2000.0, "two thousand"),
        (24.0, "24"),
        (24.0, "2 dozens"),
        (24.0, "two dozen"),
        (24.0, "Two dozen"),
        (1.1, "1.1"),
        (1.1, "1.10"),
        (1.1, "01.10"),
        (1.1, "1 point 1"),
        (0.77, ".77"),
        (0.77, "0.77"),
        (0.77, "point 77"),
        (100000.0, "100,000"),
        (100000.0, "100,000.0"),
        (100000.0, "100000"),
        (100000.0, "100K"),
        (100000.0, "100k"),
        (100000.0, "one hundred thousand"),
        (0.2, "1/5"),
        (0.2, "2/10"),
        (0.2, "3/15"),
        (0.2, "20/100"),
        (3e6, "3M"),
        (3e6, "3000K"),
        (3e6, "3000000"),
        (3e6, "3,000,000"),
        (3e6, "3 million"),
        (3e6, "30 lakh"),
        (3e6, "30 lkh"),
        (3e6, "30 l"),
        (1.2e6, "1,200,000"),
        (1.2e6, "1200000"),
        (1.2e6, "1.2M"),
        (1.2e6, "1200k"),
        (1.2e6, ".0012G"),
        (1.2e6, "12 lakhs"),
        (1.2e6, "12 lkhs"),
        (5000.0, "5 thousand"),
        (5000.0, "five thousand"),
        (-504.0, "-504"),
        (-504.0, "-negative five hundred and four"),
        (-1.2e6, "- 1,200,000"),
        (-1.2e6, "-1200000"),
        (-1.2e6, "minus 1,200,000"),
        (-1.2e6, "negative 1200000"),
        (-1.2e6, "-1.2M"),
        (-1.2e6, "-1200K"),
        (-1.2e6, "-.0012G"),
        (-3200000.0, "-3,200,000"),
        (-3200000.0, "-3200000"),
        (-3200000.0, "minus three million two hundred thousand"),
        (122.0, "one twenty two"),
        (122.0, "ONE TwentY tWO"),
        (2e5, "two Hundred thousand"),
        (21011.0, "twenty-one thousand Eleven"),
        (721012.0, "seven hundred twenty-one thousand twelve"),
        (721012.0, "seven hundred twenty-one thousand and twelve"),
        (31256721.0, "thirty-one million two hundred fifty-six thousand seven hundred twenty-one"),
        (31256721.0, "three crore twelve lakh fifty-six thousand seven hundred twenty-one"),
        (31256721.0, "three cr twelve lac fifty-six thousand seven hundred twenty-one"),
        (2400.0, "two hundred dozens"),
        (2400.0, "200 dozens"),
        (2200000.0, "two point two million"),
        (3000000000.0, "three billions"),
        (3000000000.0, "three thousand millions"),
        (45.0, "forty-five (45)"),
        (45.0, "45 (forty five)"),
    ];

    let rules = numeral::rules::get();

    for (expected, input) in cases {
        let ctx = Context::default();
        let opts = Options {};

        // Run the full parser (like `main.rs`) so composite rules can fire.
        let parser = crate::engine::Parser::new(input, &rules);
        let resolved = parser.run(&ctx, &opts);

        let mut matched = false;
        for rt in resolved.iter() {
            if rt.node.token.dim == Dimension::Numeral {
                if let TokenKind::Numeral(nd) = &rt.node.token.kind {
                    if (nd.value - expected).abs() < 1e-9 {
                        matched = true;
                        break;
                    }
                }
            }
        }

        assert!(
            matched,
            "No rule produced expected numeral {} for input '{}' (resolved: {:#?})",
            expected, input, resolved
        );
    }
}
