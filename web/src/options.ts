export class OptionValues {
  sport: number;
  speed: number;

  constructor(sport: number, speed: number) {
    this.sport = sport;
    this.speed = speed;
  }

  toJSON(): object {
    return {
      sport: this.sport,
      speed: this.speed,
    };
  }

  clone(): OptionValues {
    return new OptionValues(this.sport, this.speed);
  }

  equals(other: OptionValues): boolean {
    return this.sport === other.sport && this.speed === other.speed;
  }

  static fromJSON(json: string, defaults: OptionValues): OptionValues {
    const result = defaults.clone();
    let parsed;
    try {
      parsed = JSON.parse(json);
    } catch {
      parsed = {};
    }
    if (typeof parsed.sport === "number") {
      result.sport = parsed.sport;
    }
    if (typeof parsed.speed === "number") {
      result.speed = parsed.speed;
    }
    return result;
  }
}

function selectOption(ele: HTMLSelectElement, value: string) {
  for (let i = 0; i < ele.options.length; ++i) {
    if (ele.options[i].value === value) {
      ele.selectedIndex = i;
      return;
    }
  }
  console.warn(`Unable to set value ${value} on ${ele}`);
}

export class Options {
  resetDefaultsButton: HTMLButtonElement;
  sportElement: HTMLSelectElement;
  speedElement: HTMLInputElement;
  defaultValues: OptionValues;
  private changedCallbacks: (() => void)[];

  constructor(
    resetDefaultsButton: HTMLButtonElement,
    sportElement: HTMLSelectElement,
    speedElement: HTMLInputElement,
  ) {
    this.resetDefaultsButton = resetDefaultsButton;
    this.sportElement = sportElement;
    this.speedElement = speedElement;
    this.defaultValues = this.getCurrentValues();
    this.changedCallbacks = [];
    this.addEventListeners();
  }

  addChangedCallback(callback: () => void) {
    this.changedCallbacks.push(callback);
  }

  getCurrentValues(): OptionValues {
    const sport = Number(this.sportElement.value);
    const speed = Number(this.speedElement.value);
    return new OptionValues(sport, speed);
  }

  setValues(values: OptionValues) {
    selectOption(this.sportElement, values.sport.toString());
    this.speedElement.valueAsNumber = values.speed;
  }

  private addEventListeners() {
    this.resetDefaultsButton.addEventListener("click", () => {
      this.resetDefaults();
    });
    this.sportElement.addEventListener("change", () => {
      this.handleChanged();
    });
    this.speedElement.addEventListener("change", () => {
      this.handleChanged();
    });
  }

  resetDefaults() {
    this.setValues(this.defaultValues);
    this.saveLocally();
    this.resetDefaultsButton.disabled = true;
  }

  handleChanged() {
    this.resetDefaultsButton.disabled = false;
    this.changedCallbacks.forEach((callback) => {
      callback();
    });
    this.saveLocally();
  }

  saveLocally() {
    const json = JSON.stringify(this.getCurrentValues());
    console.log(`saveLocally: ${json}`);
    localStorage.setItem("options", json);
  }

  restoreLocally() {
    const savedString = localStorage.getItem("options");
    if (savedString === null) {
      console.log("restoreLocally: no saved value");
      return;
    }
    const saved = OptionValues.fromJSON(savedString, this.defaultValues);
    console.log(`restoreLocally: read ${savedString}, parsed ${saved}`);
    if (!saved.equals(this.defaultValues)) {
      this.resetDefaultsButton.disabled = false;
    }
    this.setValues(saved);
  }
}
