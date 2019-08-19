import React from 'react'
import {
  withStyles,
} from '@material-ui/styles'
import {
  Drawer as MaterialUIDrawer,
  Hidden,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  ListSubheader,
} from '@material-ui/core'

import Inbox from '@material-ui/icons/Inbox'
import OpenWith from '@material-ui/icons/OpenWith'
import Code from '@material-ui/icons/Code'
import Keyboard from '@material-ui/icons/Keyboard'
import Settings from '@material-ui/icons/Settings'
import Home from '@material-ui/icons/Home'

import { Link } from 'react-router-dom'
import { withRouter } from 'react-router'
import gql from 'graphql-tag'
import classnames from 'classnames'

export const DrawerFragment = gql`
  fragment DrawerFragment on Query {
    machines {
      id
      name
    }
  }
`

export const drawerWidth = 240

const styles = theme => ({
  // root: {
  //   height: '100%'
  //   width: '100%',
  //   height: 430,
  //   zIndex: 1,
  //   overflow: 'hidden',
  // },
  navIconHide: {
    [theme.breakpoints.up('md')]: {
      display: 'none',
    },
  },
  fullHeight: {
    height: '100%',
  },
  drawerRoot: {
    height: '100%',
  },
  drawerPaper: {
    width: 250,
    height: '100%',
    [theme.breakpoints.up('md')]: {
      width: drawerWidth,
      position: 'inherit',
      top: 'inherit',
    },
  },
  content: {
    backgroundColor: theme.palette.background.default,
    width: '100%',
    padding: theme.spacing(3),
    height: 'calc(100% - 56px)',
    marginTop: 56,
    [theme.breakpoints.up('sm')]: {
      height: 'calc(100% - 64px)',
      marginTop: 64,
    },
  },
  activeLink: {
    backgroundColor: '#ccc',
  },
})

const DrawerLink = withRouter(({
  classes,
  text,
  href,
  location,
  icon,
}) => (
  <ListItem
    button
    component={React.forwardRef((props, ref) => (
      <Link to={href} innerRef={ref} {...props} />
    ))}
    className={location.pathname === href ? classes.activeLink : null}
  >
    {
      icon && (
        <ListItemIcon>
          {icon}
        </ListItemIcon>
      )
    }
    <ListItemText primary={text} />
  </ListItem>
))

const DrawerContents = ({ hostIdentity, machines, classes }) => (
  <div>
    <List>
      <Hidden mdUp>
        <DrawerLink
          text="Home"
          icon={<Home />}
          href="/"
          classes={classes}
        />
      </Hidden>
      <ListSubheader>Print Queue</ListSubheader>
      <DrawerLink
        text="Print Queue"
        icon={<Inbox />}
        href={`/q/${hostIdentity.id}/`}
        classes={classes}
      />
      {
        machines.map(machine => (
          <div
            key={machine.id}
          >
            <ListSubheader>{machine.name}</ListSubheader>
            <DrawerLink
              text="Control Panel"
              icon={<OpenWith />}
              href={`/m/${hostIdentity.id}/${machine.id}/manual-control/`}
              classes={classes}
            />
            <DrawerLink
              text="Terminal"
              icon={<Keyboard />}
              href={`/m/${hostIdentity.id}/${machine.id}/terminal/`}
              classes={classes}
            />
            <DrawerLink
              text="Config"
              icon={<Settings />}
              href={`/m/${hostIdentity.id}/${machine.id}/config/`}
              classes={classes}
            />
          </div>
        ))
      }
      <ListSubheader>Dev Tools</ListSubheader>
      <DrawerLink
        text="GraphQL"
        icon={<Code />}
        href={`/q/${hostIdentity.id}/graphql-playground/`}
        classes={classes}
      />
    </List>
  </div>
)

const Drawer = ({
  hostIdentity,
  machines,
  mobileOpen,
  onClose,
  classes,
  className,
}) => (
  <div className={classnames(classes.fullHeight, className)}>
    <Hidden mdUp className={classes.fullHeight}>
      <MaterialUIDrawer
        type="temporary"
        anchor="left"
        open={mobileOpen}
        onClose={onClose}
        classes={{
          root: classes.drawerRoot,
          paper: classes.drawerPaper,
        }}
        ModalProps={{
          keepMounted: true, // Better open performance on mobile.
        }}
      >
        <DrawerContents
          hostIdentity={hostIdentity}
          machines={machines}
          classes={classes}
        />
      </MaterialUIDrawer>
    </Hidden>
    <Hidden smDown implementation="css" className={classes.fullHeight}>
      <MaterialUIDrawer
        variant="persistent"
        open
        classes={{
          root: classes.drawerRoot,
          paper: classes.drawerPaper,
        }}
      >
        <DrawerContents
          hostIdentity={hostIdentity}
          machines={machines}
          classes={classes}
        />
      </MaterialUIDrawer>
    </Hidden>
  </div>
)

export default withStyles(styles, { withTheme: true })(Drawer)