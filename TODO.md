# TODO

- [ ] Build binaries and packages installers. Setup up a release procedure.
- [ ] Add HELP.md and show it in the GUI as a pane grid component. Cover what ICOADS is. Importing data. Creating summary collection. Any warnings.
- [ ] Open help on first open.
- [ ] Fix summary collection save file. Make sure it has a .json appended if not added by the user. (This seem to only effect linux build)
- [ ] When closing the window confirm unsaved changes with a dialogue.
- [ ] When all files failed make sure nothing is written or committed.
- [ ] rfd windows parent should be set to the appropriate windows.
- [ ] When open/save as window is open disable both buttons and new button.
- [ ] Add a confirmation for cancel. And for when closing the data manager.
- [ ] Add dataset statistics to pick directory view. Maybe also add the number of duplicates based on uid.
- [ ] Add a reset to 0/0 button to globe.
- [ ] Add overall geo area of the selection to the stats.
- [ ] Add version to GUI
- [ ] Solve the memory issues on import. Need some way to control how many records to keep in memory before writing out (the current static setting is not enough it should dynamically fluctuate based on available memory)
- [ ] Move the directory selection to a separate config section.
- [ ] Create one summary per month for the same selection/name.
