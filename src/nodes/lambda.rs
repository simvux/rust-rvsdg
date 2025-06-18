use crate::{NodeKind, TranslationUnitContext, id};

impl NodeKind for Lambda {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn node_type(&self) -> &str {
        "lambda"
    }
}

#[derive(Debug)]
pub struct Lambda {}

impl TranslationUnitContext {
    // Find which region argument that corresponds to an input
    // pub fn find_lambda_input_argument(
    //     &self,
    //     lambda: id::Node<Lambda>,
    //     input: id::Input,
    // ) -> id::Argument {
    //     let in_region = self.region(lambda.id);
    //     let in_args = self.regions[in_region].arguments;
    //     let in_inputs = self.nodes[lambda.id].inputs;

    //     let in_function_args = in_args - in_inputs;

    //     let forwarded_arg = in_function_args + input.as_u32();
    //     id::Argument::from_u32(forwarded_arg)
    // }
}
