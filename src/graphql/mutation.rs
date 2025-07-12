use crate::graphql::mutations::{CreateSessionMutation, CreateUserMutation};
use async_graphql::MergedObject;

/// Root mutation object for the Dimensional Pocket Auth Service GraphQL API.
///
/// This is the main entry point for all GraphQL mutations. It combines all
/// individual mutation resolvers into a single unified interface using MergedObject.
///
/// Available mutations:
/// - createUser: Create a new user account
/// - createSession: Authenticate user and create session (sign-in)
///
/// Future mutations will be added here as the service expands to include
/// user management, authentication, and other auth-related operations.
#[derive(MergedObject, Default)]
pub struct Mutation(CreateUserMutation, CreateSessionMutation);

impl Mutation {
  pub fn new() -> Self {
    Self(CreateUserMutation, CreateSessionMutation)
  }
}
