import React from 'react'
import { compose, withProps } from 'recompose'
import { Link } from 'react-router-dom'
import {
  withStyles,
  List,
  ListItem,
  // ListItemIcon,
  ListItemText,
  Tooltip,
  Fab,
} from '@material-ui/core'
import {
  // Widgets,
  Add,
} from '@material-ui/icons'

import gql from 'graphql-tag'

import withLiveData from '../../shared/higherOrderComponents/withLiveData'

import UpdateDialog, { UPDATE_DIALOG_FRAGMENT } from '../components/UpdateDialog/Index'
import DeleteConfirmationDialog from '../components/DeleteConfirmationDialog'
import CreatePluginDialog from './create/CreatePluginDialog'

const PLUGINS_SUBSCRIPTION = gql`
  subscription ConfigSubscription($printerID: ID!) {
    live {
      patch { op, path, from, value }
      query {
        printers(printerID: $printerID) {
          id
          plugins {
            id
            package
            isEssential
          }
          availablePackages
        }
      }
    }
  }
`

const styles = theme => ({
  title: {
    paddingTop: theme.spacing.unit * 3,
  },
  addFab: {
    position: 'fixed',
    zIndex: 10,
    bottom: theme.spacing.unit * 4,
    right: theme.spacing.unit * 2,
  },
})

const enhance = compose(
  withProps(ownProps => ({
    subscription: PLUGINS_SUBSCRIPTION,
    variables: {
      printerID: ownProps.match.params.printerID,
    },
  })),
  withLiveData,
  withProps(({ printers, match }) => {
    const { pluginID, printerID, verb } = match.params
    const { plugins, availablePackages } = printers[0]

    return {
      selectedPlugin: plugins.find(c => c.id === pluginID),
      plugins,
      availablePackages,
      printerID,
      pluginID,
      verb,
    }
  }),
  withStyles(styles, { withTheme: true }),
)

const ComponentsConfigIndex = ({
  classes,
  printerID,
  plugins,
  pluginID,
  selectedPlugin,
  verb,
  availablePackages,
}) => (
  <main>
    { pluginID !== 'new' && selectedPlugin != null && verb == null && (
      <UpdateDialog
        title={selectedPlugin.package}
        open={selectedPlugin != null}
        deleteButton={selectedPlugin.isEssential === false}
        collection="PLUGIN"
        variables={{ printerID, package: selectedPlugin.package }}
        query={gql`
          query($printerID: ID!, $package: String) {
            printers(printerID: $printerID) {
              plugins(package: $package) {
                configForm {
                  ...UpdateDialogFragment
                }
              }
            }
          }
          ${UPDATE_DIALOG_FRAGMENT}
        `}
      />
    )}
    { selectedPlugin != null && verb === 'delete' && (
      <DeleteConfirmationDialog
        type={selectedPlugin.package}
        title={selectedPlugin.package}
        id={selectedPlugin.id}
        collection="PLUGIN"
        printerID={printerID}
        open={selectedPlugin != null}
      />
    )}
    <CreatePluginDialog
      printerID={printerID}
      open={pluginID === 'new'}
      availablePackages={availablePackages}
    />
    <Tooltip title="Add Plugin" placement="left">
      <Link to="new/" style={{ textDecoration: 'none' }}>
        <Fab
          component="label"
          className={classes.addFab}
        >
          <Add />
        </Fab>
      </Link>
    </Tooltip>
    <List>
      {
        plugins.map(plugin => (
          <ListItem
            button
            divider
            key={plugin.id}
            component={props => (
              <Link to={`${plugin.id}/`} {...props} />
            )}
          >
            {/*
              <ListItemIcon>
                <Widgets />
              </ListItemIcon>
            */}
            <ListItemText>
              {plugin.package}
            </ListItemText>
          </ListItem>
        ))
      }
    </List>
  </main>
)

export const Component = withStyles(styles, { withTheme: true })(
  ComponentsConfigIndex,
)
export default enhance(ComponentsConfigIndex)
