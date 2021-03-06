export default `
"""
The \`JSON\` scalar type represents JSON values as specified by [ECMA-404](http://www.ecma-international.org/publications/files/ECMA-ST/ECMA-404.pdf).
"""
scalar JSON

scalar DateTime

schema {
  query: Query
  mutation: Mutation
  subscription: Subscription
}

# Queries

type Query

# Subscriptions

type Subscription {
  live: LiveSubscriptionRoot
}

type LiveSubscriptionRoot {
  query: Query
  patch: [RFC6902Operation!]
}

type RFC6902Operation {
  op: String!
  path: String!
  from: String
  value: JSON
}

# Mutations

type Mutation
`
