import scipy.interpolate
import numpy
from plotnine import ggplot, geom_line, aes
import pandas

def main():
    max_index_value = 20
    spline_x = numpy.linspace(0, max_index_value, 20, endpoint=True)
    spline_y = numpy.exp(spline_x)
    spline = scipy.interpolate.CubicSpline(spline_x, spline_y)
    linear = scipy.interpolate.interp1d(spline_x, spline_y)
    poly = numpy.polyfit(spline_x, spline_y, deg=10)
    poly = numpy.poly1d(poly)

    plot_x = numpy.linspace(1, 10, 1000)
    exact = numpy.exp(plot_x)
    spline_approx = spline(plot_x)
    linear_approx = linear(plot_x)
    poly_approx = poly(plot_x)

    spline_error = numpy.abs(spline_approx - exact)
    linear_error = numpy.abs(linear_approx - exact)
    poly_error = numpy.abs(poly_approx - exact)

    plot_frame = pandas.DataFrame({'x': plot_x, 'spline': spline_error, 'linear': linear_error, 'poly': poly_error})
    plot_frame = pandas.melt(plot_frame, id_vars = ['x'], var_name='type', value_name='error')

    print(ggplot(plot_frame, aes('x', 'error', colour='type')) + geom_line())
    # print(spline)

if __name__ == '__main__':
    main()