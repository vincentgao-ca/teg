export default `
# Queries

extend type Query {
  devices: [Device!]!
}

type Device {
  id: ID!
  # type: String!
  connected: Boolean!
}
`
