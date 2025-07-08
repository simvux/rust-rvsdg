use crate::{Input, Origin, Output, TranslationUnitContext, User, get_kind, id, node_kind_impl};
use std::any::Any;
use std::collections::HashMap;

pub trait NodeKind: std::any::Any + std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn node_type(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct Apply {}
node_kind_impl!(Apply, "apply");

#[derive(Debug, Clone)]
pub struct DoWhile {}
node_kind_impl!(DoWhile, "theta");

#[derive(Debug, Clone)]
pub struct GlobalV {}
node_kind_impl!(GlobalV, "delta");

#[derive(Debug, Clone)]
pub struct Lambda {}
node_kind_impl!(Lambda, "lambda");

#[derive(Debug, Clone)]
pub struct Number(pub i128);
node_kind_impl!(Number, "number");

#[derive(Debug, Clone)]
pub struct Placeholder(pub &'static str);
node_kind_impl!(Placeholder, "placeholder");

#[derive(Debug, Clone)]
pub struct RecEnv {
    // pub lambdas: PrimaryMap<id::Output, id::Node<Lambda>>,
    pub lambdas: HashMap<id::AnyNode, (id::Argument, id::Output)>,
}
node_kind_impl!(RecEnv, "phi");

#[derive(Debug)]
pub struct TranslationUnit {
    pub region: id::Region,
}
node_kind_impl!(TranslationUnit, "omega");

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
    // pub fn get_lambda_in_recenv(
    //     &self,
    //     env: id::Node<RecEnv>,
    //     in_: id::Node<Lambda>,
    //     lambda: id::Node<Lambda>,
    // ) -> Result<id::Argument, id::Argument> {
    //     let env_region = self.region(env.id);
    //     let env = get_kind(&self.nodes, env);

    //     let env_lambda_argument = env
    //         .lambdas
    //         .get(&lambda.id)
    //         .expect("target lambda is not in recenv")
    //         .0;

    //     for edge in &self.regions[env_region].edges {
    //         if let Origin::Argument(arg) = edge.origin {
    //             if arg == env_lambda_argument {
    //                 if let User::Input(node_id, input) = edge.user {
    //                     if node_id == in_.id {
    //                         let arg = self.input_as_argument(Input {
    //                             node: in_,
    //                             id: input,
    //                         });
    //                         return Ok(arg);
    //                     }
    //                 }
    //             }
    //         }
    //     }

    //     Err(env_lambda_argument)
    // }
}
