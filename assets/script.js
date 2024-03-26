var eventSource;
var controller_percent = 0;

let fastest_lap = Number.MAX_VALUE;
let personal_best = Array(6).fill(Number.MAX_VALUE);
let laps = Array(6).fill(0);

function set_lights(num_lights) {

  if(num_lights > 0) {
    $('#light-6').hide();
  }
  else {
    $('#light-6').show();
  }

  for(i=1; i<=5; i++) {
    if(num_lights < i) {
      $('#light-'+i).hide();
    }
    else {
      $('#light-'+i).show();
    }
  }

}

$(document).ready(function(){
  set_lights(0);

  eventSource = new EventSource("sse");

  for(i=0; i<6; i++) {
    $('#car-'+i+'-fastest-lap').hide();
  }

  eventSource.addEventListener("Arduino", (event) => {
    const data = JSON.parse(event.data);
    console.log(data);

    if(data["NewLap"] !== undefined) {

      const car_id = data["NewLap"][0];
      const laptime = data["NewLap"][1]["secs"] + (data["NewLap"][1]["nanos"]/1000000000.0);
      let minutes = Math.floor(laptime/60);
      let seconds = (laptime % 60).toFixed(2);

      let lap_count = laps[car_id];
      lap_count += 1;
      laps[car_id] = lap_count;

      $('#car-'+car_id+'-laps').text(lap_count);

      const time_str = (minutes > 0 ? ''+minutes+'m' : '') + seconds + 's';
      
      if(personal_best[car_id] > laptime) {
        personal_best[car_id] = laptime;
        $('#car-'+car_id+'-personal-best').text(time_str);
      }

      if(fastest_lap > laptime) {
        fastest_lap = laptime;
        for(i=0; i<6; i++) {
          $('#car-'+i+'-fastest-lap').hide();
        }
        $('#car-'+car_id+'-fastest-lap').show();
      }

      $('#car-'+car_id+'-last-lap').text(time_str);
    }
    else if(data["LightUpdate"] !== undefined ) {
      const num_lights = data["LightUpdate"];
      set_lights(num_lights);
    }
    else if(data["CarUpdate"] !== undefined) {
      const car_id = data["CarUpdate"][0];
      const car_data = data["CarUpdate"][1];

      let fuel_level = car_data["fuel_level"];
      if (fuel_level >= 8)
        fuel_level -= 8;

      const fuel_percent = ((fuel_level / 7.0)*100) + "%";
      console.log(fuel_percent);
      $('#car-'+car_id+'-fuel').css('width', fuel_percent);
    }
  
  });
  
  eventSource.addEventListener("Controller", (event) => {
    const data = JSON.parse(event.data);
    const percent = (data["ControllerUpdate"][1] / 15) * 100;
  
    if (percent != controller_percent) {
      $('#controller-0').css('width', ''+percent+'%');
      controller_percent = percent;
    }
  
  });
});