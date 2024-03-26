# Carrera Webview
## Features
This project aims to bring information the carrera ditial 1:32 slotcat track to your browser, live! In order to do so, this rust webserver, utilising the axum framework, communicates with an arudino via a serial port. The messages recieved from the arduino will then be mapped to a stream and forwarded to each connected client via the SSE protocol, enabling real-time updates without the need for polling.

Right now the server offers:
 - A rudimentary user interface written in HTML5, fetching data via SSE and displaying them using JQuery
 - Start-Light events
 - New-Lap events
 - Fuel updates
 - Controller-Speed updates
 - Reset events

with the following messages planned:
 - Pitlane-Status updates
 - Safety/PaceCar-Status updates
 - Additional Information from accessories like the sector-timme-sensors need changes from the arduino side of the project
