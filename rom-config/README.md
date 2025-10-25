# ROM Configs

This directory contains configuration files that can be used to generate various ROM collections for use with One ROM.

There is a [schema file](schema.json) that describes the structure of these configuration files. You can use this schema to validate your own configuration files or to generate new ones.

If you would like a more human readable version of the schema use a tool like [json-schema.app](https://json-schema.app/view/%23?url=https%3A%2F%2Fimages.onerom.org%2Fconfigs%2Fschema.json), pasting in https://images.onerom.org/configs/schema.json as the URL.

## Minimal Config

This is a minimal config:

```json
{
    "$schema": "https://images.onerom.org/configs/schema.json",
    "version": 1,
    "description": "A minimal ROM config",
    "rom_sets": []
}
```

This produces a firmware image and metadata with no ROMs in it.  This may be useful for manufacturing purposes - in order to flash and ship One ROM with no ROMs installed.

## Simple Config

A slightly more advanced config with 2 ROM images:

```json
{
    "$schema": "https://images.onerom.org/configs/schema.json",
    "version": 1,
    "name": "Simple Config",
    "description": "A simple ROM config with 2 ROMs",
    "rom_sets": [
        {
            "type": "single",
            "description": "A simple ROM set with 2 ROMs",
            "roms": [
                {
                    "name": "Super Mario Bros.",
                    "source": {
                        "type": "local_file",
                        "path": "/path/to/super_mario_bros.nes"
                    }
                },
                {
                    "name": "The Legend of Zelda",
                    "source": {
                        "type": "local_file",
                        "path": "/path/to/legend_of_zelda.nes"
                    }
                }
            ]
        }
    ]
}
```
