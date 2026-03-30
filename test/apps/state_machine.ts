const println = (...args: any[]) => console.log(...args);
const print = (...args: any[]) => process.stdout.write(args.join(''));
const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);
const int_to_string = (n: number): string => String(n);
const string_length = (s: string): number => s.length;

type Light = { _tag: "Red" } | { _tag: "Yellow" } | { _tag: "Green" };
const Red: Light = { _tag: "Red" };
const Yellow: Light = { _tag: "Yellow" };
const Green: Light = { _tag: "Green" };

type Action = { _tag: "Next" } | { _tag: "Emergency" };
const Next: Action = { _tag: "Next" };
const Emergency: Action = { _tag: "Emergency" };

function transition(light, action) {
  switch (action._tag) {
    case "Emergency": {
      return { _tag: "Red" };
      break;
    }
    case "Next": {
      switch (light._tag) {
        case "Red": {
          return { _tag: "Green" };
          break;
        }
        case "Green": {
          return { _tag: "Yellow" };
          break;
        }
        case "Yellow": {
          return { _tag: "Red" };
          break;
        }
      }
      break;
    }
  }
}

function light_name(light) {
  switch (light._tag) {
    case "Red": {
      return "RED";
      break;
    }
    case "Yellow": {
      return "YELLOW";
      break;
    }
    case "Green": {
      return "GREEN";
      break;
    }
  }
}

function run_sequence(light, steps) {
  while (true) {
    if (steps <= 0) {
      return light;
    } else {
      println("  " + light_name(light));
      const __tco_light = transition(light, { _tag: "Next" });
      const __tco_steps = steps - 1;
      light = __tco_light;
      steps = __tco_steps;
      continue;
    }
  }
}

function main() {
  println("Traffic light sequence:");
  const final_state = run_sequence({ _tag: "Red" }, 7);
  println("Final state: " + light_name(final_state));
  println("Emergency from Green:");
  const emergency = transition({ _tag: "Green" }, { _tag: "Emergency" });
  return println("  " + light_name(emergency));
}

main();
