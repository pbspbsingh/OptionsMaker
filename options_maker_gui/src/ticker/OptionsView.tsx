export interface Options {
  calls: Option[]
  puts: Option[]
}

export interface Option {
  option_type: "CALL" | "PUT",
  symbol: string
  description: string
  strike_price: number
  expiration_date: string
  volatility: number
  delta: number
  bid: number
  bid_size: number
  ask: number
  ask_size: number
  last: number
  last_size: number
  open_interest: number
  total_volume: number
}

export interface OptionsViewProps {
  options: Options,
  currentPrice: number,
  selectedId?: string,
  onSelect: (option: Option) => void;
}

export default function OptionsView({ options, currentPrice, selectedId, onSelect }: OptionsViewProps) {
  return (
    <section className="options-view grid">
      <OptionView
        isCalls={true}
        options={options.calls}
        currentPrice={currentPrice}
        onSelect={onSelect}
        selectedId={selectedId} />
      <OptionView
        isCalls={false}
        options={options.puts}
        currentPrice={currentPrice}
        onSelect={onSelect}
        selectedId={selectedId} />
    </section>
  );
}

interface OptionViewProps {
  isCalls: boolean,
  options: Option[],
  currentPrice: number,
  selectedId?: string,
  onSelect: (option: Option) => void;
}

const OptionView = ({ isCalls, options, currentPrice, selectedId, onSelect }: OptionViewProps) => {
  const expiry = new Date(options[0].expiration_date);
  const optionChains = options.map(option => (
    <tr key={option.symbol}
      title={option.symbol}
      className={selectedId === option.symbol ? "selected" : ""}
      onClick={() => onSelect(option)}>
      <td>${option.strike_price}</td>
      <td>{option.bid.toFixed(2)} x{option.bid_size}</td>
      <td>{option.ask.toFixed(2)} x{option.ask_size}</td>
      <td>{option.last.toFixed(2)} x{option.last_size}</td>
      <td>{option.delta.toFixed(2)}</td>
      <td>{option.volatility.toFixed(2)}</td>
      <td>{option.open_interest}</td>
    </tr>
  ));

  const priceLine = (<tr className="current-price" key="currentPrice">
    <td colSpan={7}>${currentPrice.toFixed(2)}</td>
  </tr>);
  const idx = findNextGreater(options, currentPrice);
  const updatedOptionsChaing = [
    ...optionChains.slice(0, idx),
    priceLine,
    ...optionChains.slice(idx),
  ];
  if (!isCalls) {
    updatedOptionsChaing.reverse();
  }

  return (
    <article>
      <header>
        <h6>{isCalls ? "Calls" : "Puts"} - {expiry.toLocaleDateString()}</h6>
      </header>
      <table>
        <thead>
          <tr>
            <td>Strike</td>
            <td>Bid</td>
            <td>Ask</td>
            <td>Last</td>
            <td>Delta</td>
            <td>Volataility</td>
            <td title="Open interestes">Interests</td>
          </tr>
        </thead>
        <tbody>
          {updatedOptionsChaing}
        </tbody>
      </table>
    </article>
  );
}

const findNextGreater = (arr: Option[], cur: number): number => {
  for (let i = 0; i < arr.length; i++) {
    if (arr[i].strike_price >= cur) {
      return i;
    }
  }
  return 0;
}
