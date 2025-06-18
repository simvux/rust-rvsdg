use std::collections::HashMap;

use crate::{
    Input, Lambda, NodeKind, Origin, Output, TranslationUnitContext, User, get_kind, get_kind_mut,
    id,
};

impl NodeKind for RecEnv {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn node_type(&self) -> &str {
        "phi"
    }
}

// lambda regions have their function arguments first, inputs second.
//
// the phi re-forwarding of the closures will be mixed lazily among the other inputs for the lambda
// node.
#[derive(Debug)]
pub struct RecEnv {
    // pub lambdas: PrimaryMap<id::Output, id::Node<Lambda>>,
    pub lambdas: HashMap<id::AnyNode, (id::Argument, id::Output)>,
}

impl TranslationUnitContext {
    pub fn move_lambda_to_recenv(
        &mut self,
        lambda: id::Node<Lambda>,
    ) -> (id::Argument, Output<Lambda>) {
        let to = self.region;

        // TODO: it's difficult for me to imagine how this would look.
        //
        // So; let's try just using this in a real lower.

        self.move_node(lambda.id, to);
        todo!();
    }

    // Get another lambda from the recenv, if its been connected.
    //
    // Return the argument in the env region if it hasn't.
    pub fn get_lambda_in_recenv(
        &self,
        env: id::Node<RecEnv>,
        in_: id::Node<Lambda>,
        lambda: id::Node<Lambda>,
    ) -> Result<id::Argument, id::Argument> {
        let env_region = self.region(env.id);
        let env = get_kind(&self.nodes, env);

        let env_lambda_argument = env
            .lambdas
            .get(&lambda.id)
            .expect("target lambda is not in recenv")
            .0;

        for edge in &self.regions[env_region].edges {
            if let Origin::Argument(arg) = edge.origin {
                if arg == env_lambda_argument {
                    if let User::Input(node_id, input) = edge.user {
                        if node_id == in_.id {
                            let arg = self.input_as_argument(Input {
                                node: in_,
                                id: input,
                            });
                            return Ok(arg);
                        }
                    }
                }
            }
        }

        Err(env_lambda_argument)
    }
}
