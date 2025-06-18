use crate::*;
use std::io::Write;

// fn fa x = fb (x + 1)
// fn fb y = fa (y - 1)
#[test]
fn phi() {
    let (mut ctx, _) = TranslationUnitContext::new();

    let fa = ctx.add_lambda_node();
    let fa_region = ctx.region(fa.node.id);
    ctx.in_region(fa_region, |ctx| {
        let x = ctx.add_argument();
        let fb_input = ctx.add_input(fa.node);
        let fb = ctx.input_as_argument(fb_input);

        let num = ctx.add_number_node(1);

        let plus = ctx.add_placeholder_node("+");
        let plus_x = ctx.add_input(plus.node);
        let plus_y = ctx.add_input(plus.node);
        ctx.connect(x, plus_x);
        ctx.connect(num, plus_y);

        let apply = ctx.add_apply_node();
        let apply_output = ctx.add_output(apply.node);
        ctx.connect(fb, apply);

        let result = ctx.add_result();
        ctx.connect(apply_output, result);
    });

    let fb = ctx.add_lambda_node();
    let fb_region = ctx.region(fb.node.id);
    ctx.in_region(fb_region, |ctx| {
        let x = ctx.add_argument();
        let fa_input = ctx.add_input(fb.node);
        let fa = ctx.input_as_argument(fa_input);

        let num = ctx.add_number_node(1);

        let minus = ctx.add_placeholder_node("-");
        let minus_x = ctx.add_input(minus.node);
        let minus_y = ctx.add_input(minus.node);
        ctx.connect(x, minus_x);
        ctx.connect(num, minus_y);

        let apply = ctx.add_apply_node();
        let apply_output = ctx.add_output(apply.node);
        ctx.connect(fa, apply);

        let result = ctx.add_result();
        ctx.connect(apply_output, result);
    });

    let recenv = ctx.add_recenv_node();
    let recenv_region = ctx.region(recenv.id);

    ctx.move_node(fa.node.id, recenv_region);
    ctx.move_node(fb.node.id, recenv_region);

    let [fa_out, _fb_out] = ctx.in_region(recenv_region, |ctx| {
        let (_fa_arg, fa_out) = ctx.move_lambda_to_recenv(fa.node);
        let (_fb_arg, fb_out) = ctx.move_lambda_to_recenv(fb.node);

        [fa_out, fb_out]
    });

    let main = ctx.add_lambda_node();
    let main_region = ctx.region(main.node.id);
    ctx.in_region(main_region, |ctx| {
        let main_fa_input = ctx.add_input(main.node);
        let main_fa_arg = ctx.input_as_argument(main_fa_input);
        ctx.connect(fa_out, main_fa_input);

        let init = ctx.add_number_node(10);
        let apply = ctx.add_apply_node();
        ctx.connect(main_fa_arg, apply);
        ctx.connect(init, apply);

        let apply_output = ctx.add_output(apply.node);
        let result = ctx.add_result();
        ctx.connect(apply_output, result);
    });

    export(&ctx);
}

fn export(ctx: &TranslationUnitContext) {
    let xml = ctx.to_xml();
    let mut path = std::env::temp_dir();
    path.push("rvsdg.xml");
    let mut f = std::fs::File::create(&path).unwrap();
    write!(f, "{}", xml).unwrap();
    println!(" wrote to {}", path.display());

    std::process::Command::new("rvsdg-viewer")
        .arg(path)
        .spawn()
        .unwrap();
}
