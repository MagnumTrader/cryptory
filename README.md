# Cryptory

Crypto-History

CLI tool for fetching historical data for cryptocurrencies.
For now (and maybe forever) this data gets fetched from binance: [https://data.binance.vision/].

> [!WARNING] 
> I have no affiliation with Binance, and this CLI comes with no guarantees regarding data accuracy.
> I take no responsibility for any financial loss or mental distress resulting from the use of this tool.

## Installing Cryptory
Use cargo to install the tool on your computer.
```sh
cargo install --git https://github.com/MagnumTrader/cryptory.git --root <YOUR INSTALL PATH>
```
After installing the tool you can use it from the commandline (If paths is correctly setup)

## Usage

### Single file downloads
A single day with 5m timeframe
this will download 5m data for 2025-01-01
```sh
cryptory btcusdt -t 5m daily 2025-01-01
```

A single month with 1h timeframe
```sh
cryptory btcusdt -t 1h month 2025-01
```
cryptory ignore dates when the monthly command is used so this is also valid:
```sh
cryptory btcusdt -t 1h month 2025-01-01
```

### Multiple symbols

To download data for multiple tickers you specify them first.
this will download 5m data for both btcusdt and ethusdt for 2025-01-01
```sh
cryptory btcusdt ethusdt -t 5m daily 2025-01-01
```

### Multiple days
To download multiple periods you need to specify an end date/month (argument -e)

Multiple days 
this will download 5m data from 2025-01-01 to 2025-01-15 inclusive.
```sh
cryptory btcusdt -t 5m daily 2025-01-01 -e 2025-01-15
```

Multiple months
this will download 5m data from 2025-01-01 to 2025-02-28 inclusive.
```sh
cryptory btcusdt -t 5m monthly 2025-01 -e 2025-02
```
cryptory ignore dates when the monthly command is used so this is equivalent to:
```sh

cryptory btcusdt -t 5m monthly 2025-01-01 -e 2025-02-15
```

