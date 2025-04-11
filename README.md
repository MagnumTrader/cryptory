# Cryptory

Crypto-History

CLI tool for fetching historical data for cryptocurrencies.\
This data gets fetched from binance: https://data.binance.vision/ \



> [!WARNING] 
> I have no affiliation with Binance, and this CLI comes with no guarantees regarding data accuracy.\
> I take no responsibility for any financial loss or mental distress resulting from the use of this tool.




> Pull requests are welcomed!



The downloads are in zip format, after downloading you use other tools to unzip and merge files / write to your local db.

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

A single month with 1h timeframe, from 2025-01-01 to 2025-01-31 in a single file
```sh
cryptory btcusdt -t 1h month 2025-01
```
cryptory ignore dates when the monthly command is used so this has the same effect.
```sh
cryptory btcusdt -t 1h month 2025-01-01
```

### Multiple symbols

To download data for multiple tickers you specify them first.
this will download 5m data for both btcusdt and ethusdt for 2025-01-01
```sh
cryptory btcusdt ethusdt -t 5m daily 2025-01-01
```

### Multiple periods
To download multiple periods you need to specify an end date/month (argument -e)

Multiple days 
this will download 5m data from 2025-01-01 to 2025-01-15 inclusive in 15 files.
```sh
cryptory btcusdt -t 5m daily 2025-01-01 -e 2025-01-15
```

Multiple months
this will download 5m data from 2025-01-01 to 2025-02-28 inclusive into two files, one for 2025-01 and one for 2025-02.
```sh
cryptory btcusdt -t 5m monthly 2025-01 -e 2025-02
```
and as stated earlier you can use the date, but it will have no effect
```sh
cryptory btcusdt -t 5m monthly 2025-01-01 -e 2025-02-15
```
### Valid timeframes

Available timeframes are: 
1s 1m 3m 5m 15m 30m 1h 2h 4h 6h 8h 12h 1d 1w 1mo
