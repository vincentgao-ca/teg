/*
 * Intercepts DESPOOL and SPOOL actions and sends the current gcode line to the
 * printer. Executes after the reducers have resolved which line to send.
 *
 * The first SPOOL action to an idle printer triggers this to begin printing
 * the spooled line.
 */
 const txMiddleware = ({txParser}) => store => next => action => {

  if (
    action.type === 'DESPOOL' ||
    action.type === 'SPOOL' && store.getState().spool.currentLine == null
  ) {
    next(action)
    const data = store.getState().spool.currentLine
    store.dispatch({
      type: 'SERIAL_SEND',
      data,
      parsedData: txParser(data),
    })
  } else {
    next(action)
  }
}

export default txMiddleware