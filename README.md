# Options Maker
Intially started this project to automatically trade 0DTE SPX Iron condor contracts; however changed the scope of project and now it works as trade signal app.
Subscribes to the Underlying's ticker data using Schwab API:
* Sends notification based on when a price penetrates and retests a support/resistance level.
* Looks for divergences between Price and various indicators (RSI, Stocastic, etc).
