from PIL import Image

image = Image.open('Terminal.png')
width, height = image.size
data = image.load()
for y in range(0, height):
    line = ''
    color = None
    for x in range(0, int(width * 1.5)):
        pixel = data[int(x / 1.5), y]
        if color == None or color != pixel:
            line += '\\x1c{0:02x}{0:02x}{0:02x}'.format(pixel[0], pixel[1], pixel[2], pixel[3])
            color = pixel
        line += ' '
    line += '\\x19'
    print(line)
