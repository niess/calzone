# Exporting shapes from a FreeCAD model

This example demonstrates the process of exporting a FreeCAD model to Calzone.
The [example.FCStd](example.FCStd) stylesheet showcases a shielding structure
created by assembling lead bricks. Utilising the [export.py](export.py) script,
the final `LeadCastle` shape can be exported as follows:


```bash
python3 export.py -l LeadCastle
```

This assumes that FreeCAD has been appropriately configured (see below).


> [!NOTE]
>
> The provided [export.py](export.py) script is for illustrative purposes only.
> Depending on the structure of your CAD geometry, it may require modifications.


## Configuring FreeCAD

To run the [export.py](export.py) script, you will need to use a Python
interpreter that can import FreeCAD as an external package. On a Linux operating
system, this can be achieved by extracting the FreeCAD AppImage as follows:


```bash
# Extract FreeCAD.
./FreeCAD.AppImage --appimage-extract > /dev/null && mv squashfs-root FreeCAD.AppDir

# Symlink FreeCAD's Python interpreter (for conveniency).
ln -s FreeCAD.AppDir/usr/bin/python python3

# Add the FreeCAD.so package to the PYTHONPATH.
export PYTHONPATH=$PWD/FreeCAD.AppDir/usr/lib:$PYTHONPATH

# Run the example script through FreeCAD's Python.
./python3 export.py -l LeadCastle
```


[AppImage]: https://appimage.org/
[FreeCAD]: https://www.freecad.org/
